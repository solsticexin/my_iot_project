//! DHT11温湿度传感器驱动模块
//!
//! 本模块实现了DHT11温湿度传感器的异步驱动，包括传感器唤醒、响应检查、
//! 数据读取和校验功能。模块使用embassy框架，通过异步任务定期读取传感器数据
//! 并通过通道发送给其他任务。

use defmt::{error, info};
use embassy_stm32::gpio::Flex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal::delay::DelayNs;

/// DHT11传感器操作中可能出现的错误类型
#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum Dh11Error {
    /// 超时错误：传感器未在规定时间内响应
    TimeOut,
    /// 校验和错误：接收到的数据校验失败
    ChecksumError,
    /// 时间异常：脉冲宽度不在预期范围内
    TimeAnomaly,
}
/**
 * DHT11传感器异步任务函数
 *
 * 定期唤醒DHT11传感器，读取温湿度数据，并将数据通过通道发送给其他任务。
 * 任务会持续运行，每2秒尝试读取一次传感器数据。
 *
 * @param pin 连接到DHT11传感器的GPIO引脚
 * @param sender 用于发送传感器数据的通道发送器
 */
#[embassy_executor::task]
pub async fn dh11_task(
    mut pin: Flex<'static>,
    sender: embassy_sync::channel::Sender<'static, CriticalSectionRawMutex, [u8; 5], 2>,
) {
    loop {
        // 唤醒DHT11传感器
        wake_up_sensor(&mut pin).await;

        // 检查传感器响应
        if check_sensor_response(&mut pin).is_err() {
            // 响应失败，等待2秒后重试
            Timer::after(Duration::from_secs(2)).await;
            continue;
        }

        // 读取传感器数据
        match dh11_read(&mut pin) {
            Ok(data) => {
                // 数据读取成功，记录日志
                info!("dh11_read: {},{},{},{}", data[0], data[1], data[2], data[3]);
            }
            Err(e) => {
                // 数据读取失败，记录错误
                error!("dh11_read error: {}", e);
            }
        };

        // 等待2秒后再次读取
        Timer::after(Duration::from_secs(2)).await;
    }
}

/**
 * 唤醒DHT11传感器
 *
 * 通过将GPIO引脚拉低20毫秒，然后拉高45微秒来唤醒DHT11传感器。
 * 这是DHT11通信协议的起始信号。
 *
 * @param pin 连接到DHT11传感器的GPIO引脚
 */
async fn wake_up_sensor(pin: &mut Flex<'_>) {
    pin.set_low();
    Timer::after(Duration::from_millis(20)).await;
    pin.set_high();
    let mut delay = Delay;
    delay.delay_us(45);
}

/**
 * 检查DHT11传感器响应
 *
 * 在发送唤醒信号后，检查DHT11传感器的响应信号是否符合协议规范。
 * 传感器应先拉低约80微秒，然后拉高约80微秒作为响应。
 *
 * @param pin 连接到DHT11传感器的GPIO引脚
 * @return 成功返回Ok(())，失败返回Dh11Error
 */
fn check_sensor_response(pin: &mut Flex<'_>) -> Result<(), Dh11Error> {
    let low_pulse = match measure_pulse_width(pin, false, 200) {
        Ok(pulse) => pulse,
        Err(_) => {
            error!("等待低电平超时");
            return Err(Dh11Error::TimeOut);
        }
    };
    if low_pulse < 20 || low_pulse > 100 {
        error!("低电平相应异常:{}us", low_pulse);
        return Err(Dh11Error::TimeAnomaly);
    }
    let high_pulse = match measure_pulse_width(pin, true, 200) {
        Ok(pulse) => pulse,
        Err(_) => {
            error!("等待高电平超时");
            return Err(Dh11Error::TimeOut);
        }
    };
    if high_pulse < 20 || high_pulse > 100 {
        error!("高电平响应异常: {} us", high_pulse);
        return Err(Dh11Error::TimeAnomaly);
    }
    info!("dh11握手成功");
    Ok(())
}
/**
 * 测量指定电平的脉冲宽度
 *
 * 等待指定电平出现，然后测量该电平持续的时间，直到电平变化。
 *
 * @param pin 连接到DHT11传感器的GPIO引脚
 * @param level_to_measure 要测量的电平(true为高电平，false为低电平)
 * @param timeout_us 超时时间(微秒)
 * @return 成功返回脉冲宽度(微秒)，失败返回Dh11Error
 */
fn measure_pulse_width(
    pin: &mut Flex<'_>,
    level_to_measure: bool,
    timeout_us: u64,
) -> Result<u64, Dh11Error> {
    // 等待指定电平出现
    wait_for_level(pin, level_to_measure, timeout_us)?;

    // 记录当前时间
    let start = Instant::now();

    // 等待电平变化
    wait_for_level(pin, !level_to_measure, timeout_us)?;

    // 返回脉冲宽度(微秒)
    Ok(start.elapsed().as_micros())
}
/**
 * 等待GPIO引脚达到指定电平
 *
 * 持续检查GPIO引脚的电平，直到达到目标电平或超时。
 *
 * @param pin 要检查的GPIO引脚
 * @param target 目标电平(true为高电平，false为低电平)
 * @param timeout_us 超时时间(微秒)
 * @return 成功返回Ok(())，超时返回Dh11Error::TimeOut
 */
fn wait_for_level(pin: &mut Flex<'_>, target: bool, timeout_us: u64) -> Result<(), Dh11Error> {
    // 记录开始时间
    let start = Instant::now();

    // 循环检查电平
    loop {
        // 检查当前电平是否达到目标电平
        if pin.is_high() == target {
            return Ok(());
        }

        // 检查是否超时
        if start.elapsed().as_micros() > timeout_us {
            return Err(Dh11Error::TimeOut);
        }
    }
}

/**
 * 从DHT11传感器读取温湿度数据
 *
 * 读取DHT11传感器发送的40位数据(5字节)，并进行校验和验证。
 * 数据格式：
 * - 字节0: 湿度整数部分
 * - 字节1: 湿度小数部分(通常为0)
 * - 字节2: 温度整数部分
 * - 字节3: 温度小数部分(通常为0)
 * - 字节4: 校验和(前4字节之和的低8位)
 *
 * @param pin 连接到DHT11传感器的GPIO引脚
 * @return 成功返回5字节数据数组，失败返回Dh11Error
 */
fn dh11_read(pin: &mut Flex<'_>) -> Result<[u8; 5], Dh11Error> {
    let mut bytes = [0u8; 5];

    // 读取40位数据
    for bit_index in 0..40 {
        // 等待数据位的起始高电平
        if wait_for_level(pin, true, 100).is_err() {
            error!("读取数据位 {} 时等待高电平超时", bit_index);
            return Err(Dh11Error::TimeOut);
        }

        // 记录高电平开始时间
        let start = Instant::now();

        // 等待高电平结束
        if wait_for_level(pin, false, 100).is_err() {
            error!("读取数据位 {} 时等待低电平超时", bit_index);
            return Err(Dh11Error::TimeOut);
        }

        // 测量高电平持续时间
        let pulse = start.elapsed().as_micros();

        // 根据高电平持续时间判断数据位(>30us为1，否则为0)
        let bit = if pulse > 30 { 1 } else { 0 };

        // 将数据位存储到对应的字节中
        bytes[(bit_index / 8) as usize] <<= 1;
        bytes[(bit_index / 8) as usize] |= bit;
    }

    // 计算校验和
    let sum: u16 = bytes[0] as u16 + bytes[1] as u16 + bytes[2] as u16 + bytes[3] as u16;

    // 验证校验和
    if (sum & 0xFF) as u8 != bytes[4] {
        error!("校验和错误: 计算值={}, 实际值={}", sum & 0xFF, bytes[4]);
        return Err(Dh11Error::ChecksumError);
    }

    // 返回有效数据
    Ok(bytes)
}
