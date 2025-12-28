use crate::esp01s::{self, CommandExecuteFrame};
use defmt::{info, warn};
use embassy_futures::select::{Either, select};
use embassy_stm32::mode;
use embassy_stm32::usart::Uart;
use embassy_time::{Duration, Timer};

/// 手动模式任务：接收串口命令并控制继电器
#[embassy_executor::task]
pub async fn manually(uart: Uart<'static, mode::Async>, mut relay: esp01s::Relay<'static>) {
    let mut uart = esp01s::Esp01s::new(uart);
    loop {
        let mut buffer = [0u8; 128];
        let len = match uart.usart.read_until_idle(&mut buffer).await {
            Ok(len) => len,
            Err(_) => {
                warn!("串口读取数据失败！");
                continue;
            }
        };
        if len == 0 {
            info!("没有读到数据！")
        }
        // 只处理有数据的情况
        if len > 0 {
            // 将读取到的数据转换为字符串切片
            let data_str = match core::str::from_utf8(&buffer[..len]) {
                Ok(s) => s,
                Err(_) => {
                    // UTF-8解码错误，继续下一个循环
                    info!("UTF-8解码错误");
                    continue;
                }
            };
            // 反序列化为CommandExecuteFrame结构体
            match serde_json_core::from_str::<CommandExecuteFrame>(data_str) {
                Ok((command_frame, _)) => {
                    let command = esp01s::FrameType::CommandExecute(command_frame);
                    match uart.command_execute(command, &mut relay).await {
                        Ok(receipt) => {
                            // 更新继电器状态到全局共享状态
                            {
                                let mut relay_states = crate::config::RELAY_STATES.lock().await;
                                match receipt.target {
                                    esp01s::Target::Water => {
                                        relay_states.water =
                                            matches!(receipt.action, esp01s::Action::On);
                                    }
                                    esp01s::Target::Light => {
                                        relay_states.light =
                                            matches!(receipt.action, esp01s::Action::On);
                                    }
                                    esp01s::Target::Fan => {
                                        relay_states.fan =
                                            matches!(receipt.action, esp01s::Action::On);
                                    }
                                    esp01s::Target::Buzzer => {
                                        relay_states.buzzer =
                                            matches!(receipt.action, esp01s::Action::On);
                                    }
                                }
                            }

                            // 发送执行回执
                            match uart
                                .execution_receipt(esp01s::FrameType::ExecutionReceipt(receipt))
                                .await
                            {
                                Ok(_) => (),
                                Err(_) => warn!("回执发送失败"),
                            }
                        }
                        Err(_e) => {
                            warn!("FrameTypeError");
                            continue;
                        }
                    }
                }
                Err(_e) => {
                    warn!("反序列化失败");
                    continue;
                }
            }
        }
    }
}

/// 混合模式任务：同时支持自动数据上报和接收控制命令
#[embassy_executor::task]
pub async fn hybrid(usart: Uart<'static, mode::Async>, mut relay: esp01s::Relay<'static>) {
    let mut uart = esp01s::Esp01s::new(usart);

    loop {
        // 使用 select 在两个异步操作之间选择
        match select(
            // 操作1: 等待10秒后自动上报数据
            Timer::after(Duration::from_secs(10)),
            // 操作2: 监听串口接收命令
            read_command(&mut uart),
        )
        .await
        {
            // 定时器触发 - 执行自动数据上报
            Either::First(_) => {
                if let Err(e) = auto_report_data(&mut uart).await {
                    warn!("自动数据上报失败: {:?}", e);
                }
            }
            // 串口接收到数据 - 执行手动控制
            Either::Second(command_result) => match command_result {
                Ok(command) => {
                    if let Err(e) = execute_command(&mut uart, command, &mut relay).await {
                        warn!("命令执行失败: {:?}", e);
                    }
                }
                Err(e) => {
                    warn!("命令读取失败: {:?}", e);
                }
            },
        }
    }
}

/// 从串口读取命令帧
async fn read_command(
    uart: &mut esp01s::Esp01s<'_>,
) -> Result<esp01s::CommandExecuteFrame, ReadError> {
    let mut buffer = [0u8; 128];
    let len = uart
        .usart
        .read_until_idle(&mut buffer)
        .await
        .map_err(|_| ReadError::UartError)?;

    if len == 0 {
        return Err(ReadError::EmptyData);
    }

    // UTF-8解码
    let data_str = core::str::from_utf8(&buffer[..len]).map_err(|_| ReadError::Utf8Error)?;

    // JSON反序列化
    let (command_frame, _) = serde_json_core::from_str::<esp01s::CommandExecuteFrame>(data_str)
        .map_err(|_| ReadError::JsonError)?;

    Ok(command_frame)
}

/// 执行控制命令并发送回执
async fn execute_command(
    uart: &mut esp01s::Esp01s<'_>,
    command: esp01s::CommandExecuteFrame,
    relay: &mut esp01s::Relay<'_>,
) -> Result<(), ExecuteError> {
    // 执行继电器动作
    let receipt = relay.execute_action(command.target, command.action).await;

    // 更新全局继电器状态
    {
        let mut relay_states = crate::config::RELAY_STATES.lock().await;
        match receipt.target {
            esp01s::Target::Water => {
                relay_states.water = matches!(receipt.action, esp01s::Action::On);
            }
            esp01s::Target::Light => {
                relay_states.light = matches!(receipt.action, esp01s::Action::On);
            }
            esp01s::Target::Fan => {
                relay_states.fan = matches!(receipt.action, esp01s::Action::On);
            }
            esp01s::Target::Buzzer => {
                relay_states.buzzer = matches!(receipt.action, esp01s::Action::On);
            }
        }
    }

    // 发送执行回执
    uart.execution_receipt(esp01s::FrameType::ExecutionReceipt(receipt))
        .await
        .map_err(|_| ExecuteError::SendError)?;

    info!("命令执行成功并发送回执");
    Ok(())
}

/// 自动上报传感器数据
async fn auto_report_data(uart: &mut esp01s::Esp01s<'_>) -> Result<(), ReportError> {
    // 从全局状态读取传感器数据
    let (temp, humi, soil, lux) = {
        let sensor_data = crate::config::SENSOR_DATA.lock().await;
        (
            sensor_data.temperature,
            sensor_data.humidity,
            ((sensor_data.soil_moisture as u32 * 100) / 4095) as u8,
            sensor_data.light_intensity,
        )
    };

    // 读取继电器状态
    let (water_state, light_state, fan_state, buzzer_state) = {
        let relay_states = crate::config::RELAY_STATES.lock().await;
        (
            relay_states.water,
            relay_states.light,
            relay_states.fan,
            relay_states.buzzer,
        )
    };

    // 构建数据上报帧
    let frame = esp01s::DataReportFrame::new(
        temp,
        humi,
        soil,
        lux,
        water_state,
        light_state,
        fan_state,
        buzzer_state,
    );

    // 发送数据
    uart.data_report(frame)
        .await
        .map_err(|_| ReportError::SendError)?;

    info!("自动数据上报成功");
    Ok(())
}

// 错误类型定义
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ReadError {
    UartError,
    EmptyData,
    Utf8Error,
    JsonError,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ExecuteError {
    SendError,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ReportError {
    SendError,
}
