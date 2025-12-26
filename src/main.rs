#![no_std]
#![no_main]

mod bh1750;
mod config;
mod dht11;
mod esp01s;
mod fmt;
mod soil;
mod st7735;

use core::str::FromStr;
use defmt::{error, info};
// mod st7735_async;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::RgbColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
use serde_json_core::heapless::String;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Flex, Level, Output, Speed},
    i2c::I2c,
    spi::{self, Spi},
    time::{khz, mhz},
};
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    //===============================
    //初始化全局配置
    //===============================
    let config = config::stm_config();
    let p = embassy_stm32::init(config);
    //发送,接收
    let sender = config::CHANNEL.sender();
    let _receiver = config::CHANNEL.receiver();
    //===============================
    //配置dh11
    //===============================
    let mut dh11_pin = Flex::new(p.PB11);
    dh11_pin.set_as_input_output(Speed::VeryHigh);
    //===============================
    //配置st7735

    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(15);
    // let spi = Spi::new_blocking(p.SPI1, p.PA5, p.PA7, p.PA6, spi_config);
    let spi_async = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA1_CH3, p.DMA1_CH2, spi_config,
    );
    // 配置控制引脚
    // CS -> PA4, DC -> PB1, RES -> PB0
    let cs = Output::new(p.PA3, Level::Low, Speed::VeryHigh);
    let dc = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    let rst = Output::new(p.PA2, Level::Low, Speed::VeryHigh);
    // let display = st7735::init_screen(spi, dc, rst);
    //===============================

    //===============================
    //IIC引脚配置 ，BH1750传感器
    let mut i2c_config = embassy_stm32::i2c::Config::default();
    i2c_config.frequency = khz(100);
    let i2c_bh1750 = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        config::Irqs,
        p.DMA1_CH6,
        p.DMA1_CH7,
        i2c_config,
    );

    //===============================
    //配置ADC for soil sensor
    let adc = embassy_stm32::adc::Adc::new(p.ADC1);

    //===============================

    //===============================
    //串口配置
    let mut _usart1_config = embassy_stm32::usart::Config::default();
    _usart1_config.baudrate = 115200; //设置波特率
    _usart1_config.data_bits = embassy_stm32::usart::DataBits::DataBits8; //设置数据位为8位
    _usart1_config.stop_bits = embassy_stm32::usart::StopBits::STOP1; //设置停止位为1位
    _usart1_config.parity = embassy_stm32::usart::Parity::ParityNone; //设置无校验位
    let usart = embassy_stm32::usart::Uart::new(
        p.USART1,
        p.PA10,
        p.PA9,
        config::Irqs,
        p.DMA1_CH4,
        p.DMA1_CH5,
        _usart1_config,
    );
    let usart = match usart {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to create Uart: {}", e);
            return;
        }
    };

    //===============================
    //执行dh11任务
    // match spawner.spawn(dht11::dh11_task(dh11_pin, sender)) {
    //     Ok(_) => (),
    //     Err(e) => {
    //         error!("Failed to spawn task: {}", e);
    //     }
    // }
    // match spawner.spawn(test_st7735_task(spi_async, dc, rst, cs)) {
    //     Ok(_) => (),
    //     Err(e) => {
    //         error!("Failed to spawn test_st7735_task: {}", e);
    //     }
    // }
    // match spawner.spawn(bh1750::bh1750_read(i2c_bh1750)) {
    //     Ok(_) => (),
    //     Err(e) => {
    //         error!("Failed to spawn bh1750_read task: {}", e);
    //     }
    // }
    // match spawner.spawn(soil::soil(adc, p.PA0)) {
    //     Ok(_) => (),
    //     Err(e) => {
    //         error!("Failed to spawn soil task: {}", e);
    //     }
    // }
    match spawner.spawn(_test_usart_task(usart)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn usart_tack task: {}", e);
        }
    }
    //===============================
}

//===============================
//draw任务
//===============================
// #[embassy_executor::task]
// async fn draw_task(
//     mut display: st7735::St7735Display,
//     receiver: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, [u8; 5], 2>,
// ) {
//     loop {
//         let data = receiver.receive().await;
//         let hum_int = data[0];
//         let temp_int = data[2];

//         // --- 可视化显示 (画条形图) ---

//         // 1. 清除旧的图形 (用黑色矩形覆盖)
//         Rectangle::new(Point::new(10, 20), Size::new(100, 60))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
//             .draw(&mut display)
//             .unwrap();

//         // 2. 画温度条 (红色) - 长度根据温度值变化
//         let temp_len = (temp_int as u32).min(100) * 2; // 放大一点便于观察
//         Rectangle::new(Point::new(10, 30), Size::new(temp_len, 10))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
//             .draw(&mut display)
//             .unwrap();

//         // 3. 画湿度条 (青色)
//         let hum_len = (hum_int as u32).min(100);
//         Rectangle::new(Point::new(10, 50), Size::new(hum_len, 10))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
//             .draw(&mut display)
//             .unwrap();
//         Timer::after(Duration::from_secs(2)).await
//     }
// }
#[embassy_executor::task]
async fn _test_usart_task(
    mut usart: embassy_stm32::usart::Uart<'static, embassy_stm32::mode::Async>,
) {
    use embassy_time::with_timeout;

    info!("=== 串口测试任务启动 ===");
    info!("硬件连接检查清单:");
    info!("  [1] STM32 PA9  (TX) -> USB串口 RX");
    info!("  [2] STM32 PA10 (RX) -> USB串口 TX");
    info!("  [3] GND -> GND");
    info!("  [4] 波特率: 115200, 8N1");
    info!("========================");

    loop {
        // 发送测试
        info!(">>> 发送: hello world!");
        match usart.write(b"hello world!\r\n").await {
            Ok(_) => info!("✓ 发送成功"),
            Err(e) => {
                error!("✗ 发送失败: {}", e);
                Timer::after(Duration::from_secs(1)).await;
                continue;
            }
        }

        // 接收测试 - 回显测试（已注释）
        // info!("<<< 等待接收数据 (超时 2 秒)...");
        // let mut buffer = [0u8; 128];
        //
        // // 使用 read_until_idle() - 等待数据接收完整后处理
        // match with_timeout(Duration::from_secs(2), usart.read_until_idle(&mut buffer)).await {
        //     Ok(Ok(bytes_read)) => {
        //         info!("✓ 成功接收 {} 字节!", bytes_read);
        //
        //         if bytes_read > 0 {
        //             // 尝试解析为 UTF-8 字符串
        //             if let Ok(s) = core::str::from_utf8(&buffer[..bytes_read]) {
        //                 info!("接收内容: {}", s);
        //                 let _ = usart.write(b"Echo: ").await;
        //                 let _ = usart.write(&buffer[..bytes_read]).await;
        //                 let _ = usart.write(b"\r\n").await;
        //             } else {
        //                 info!("非 UTF-8 数据");
        //                 let _ = usart.write(b"[Binary Data]\r\n").await;
        //             }
        //         }
        //     }
        //     Ok(Err(e)) => {
        //         error!("✗ 读取错误: {}", e);
        //         let _ = usart.write(b"RX Error\r\n").await;
        //     }
        //     Err(_) => {
        //         info!("✗ 接收超时 - 未收到数据");
        //         let _ = usart.write(b"RX Timeout\r\n").await;
        //     }
        // }
        //
        // // 添加小延时，避免循环过快
        // Timer::after(Duration::from_millis(100)).await;
        // info!("Received: {}", received_str);
        // usart.write(b"Hello, World!\r\n").await.unwrap();
        // let mut data =esp01s::DataReportFrame::new(25, 60, 30, 500, true, false, true, false);
        // let mut json = data.to_json();
        // json.push_str("\r\n").unwrap();
        // usart.write(json.as_bytes()).await.unwrap();
        // info!("Sent: Hello, World!\r\n");
        // let _a=esp01s::Action::Off;
        // info!("{:?}",_a);

        // 测试反序列化
        // 测试 Action 反序列化
        // let action_json = r#"{"target":"Water","action":"On"}"#;
        // match serde_json_core::from_str::<esp01s::CommandExecuteFrame>(action_json) {
        //     Ok(command) => {
        //         info!("反序列化成功: {:?}", command);
        //     }
        //     Err(_) => {
        //         info!("反序列化失败:");
        //     }
        // }
        //
        // // 测试带脉冲的 Action 反序列化
        // let pulse_action_json = r#"{"target":"Fan","action":"Pulse(500)"}"#;
        // match serde_json_core::from_str::<esp01s::CommandExecuteFrame>(pulse_action_json) {
        //     Ok(command) => {
        //         info!("脉冲反序列化成功: {:?}", command);
        //     }
        //     Err(_) => {
        //         info!("脉冲反序列化失败:");
        //     }
        // }
        //
        // // 测试 Action 直接反序列化
        // let simple_action_json = r#""On""#;
        // match serde_json_core::from_str::<esp01s::Action>(simple_action_json) {
        //     Ok(action) => {
        //         info!("Action 反序列化成功: {:?}", action);
        //     }
        //     Err(_) => {
        //         info!("Action 反序列化失败:");
        //     }
        // }

        // 测试串口接收和反序列化
        info!("等待串口 JSON 数据...");
        let mut buffer = [0u8; 128];
        match with_timeout(Duration::from_secs(5), usart.read_until_idle(&mut buffer)).await {
            Ok(Ok(bytes_read)) => {
                info!("接收到 {} 字节数据", bytes_read);

                if bytes_read > 0 {
                    // 将接收到的字节转换为字符串
                    match core::str::from_utf8(&buffer[..bytes_read]) {
                        Ok(received_str_raw) => {
                            let received_str = received_str_raw.trim();
                            info!("接收到字符串: '{}'", received_str);

                            // 尝试反序列化接收到的 JSON 数据
                            // 首先尝试 CommandExecuteFrame
                            match serde_json_core::from_str::<esp01s::CommandExecuteFrame>(
                                received_str,
                            ) {
                                Ok((command, _)) => {
                                    info!("✓ 反序列化 CommandExecuteFrame 成功!");
                                    info!("解析结果 struct: {:?}", command);
                                    let _ =
                                        usart.write(b"OK: CommandExecuteFrame Received\r\n").await;
                                }
                                Err(_) => {
                                    // 如果 CommandExecuteFrame 失败，尝试 Action
                                    match serde_json_core::from_str::<esp01s::Action>(received_str)
                                    {
                                        Ok((action, _)) => {
                                            info!("✓ 反序列化 Action 成功!");
                                            info!("解析结果 enum: {:?}", action);
                                            let _ = usart.write(b"OK: Action Received\r\n").await;
                                        }
                                        Err(_) => {
                                            error!("✗ 反序列化失败");
                                            let _ =
                                                usart.write(b"Error: JSON parse failed\r\n").await;
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            error!("接收到的数据不是有效的 UTF-8 字符串");
                            let _ = usart.write(b"Error: Not UTF-8\r\n").await;
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                error!("串口读取失败: {}", e);
                let _ = usart.write(b"Error: UART read failed\r\n").await;
            }
            Err(_) => {
                info!("等待超时，未收到数据");
            }
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
#[embassy_executor::task]
async fn test_st7735_task(
    spi: Spi<'static, embassy_stm32::mode::Async>,
    dc: Output<'static>,
    rst: Output<'static>,
    cs: Output<'static>,
) {
    let mut display = st7735::ST7735::new(spi, rst, dc, cs);
    display.init().await;

    // 1. 设置方向 ( Landscape)
    display
        .set_orientation(st7735::Orientation::Landscape)
        .await;

    // 2. 清屏 (蓝色背景)
    display.clear(Rgb565::BLUE).await;

    // 3. 画圆 (黄色, 居中) - Landscape 模式下通常宽160, 高128
    // 中心点 (80, 64)
    let line_style = PrimitiveStyle::with_stroke(Rgb565::YELLOW, 2);
    let circle = Circle::new(Point::new(80, 64), 40).into_styled(line_style);

    display.draw_pixels(circle.pixels()).await;

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
