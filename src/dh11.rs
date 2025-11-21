use defmt::{error, info};
use embassy_stm32::gpio::Flex;
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal::delay::DelayNs;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum Dh11Error {
    TimeOut,
    ChecksumError,
    TimeAnomaly,
}
pub async fn wake_up_sensor(pa5: &mut Flex<'_>) {
    pa5.set_low();
    Timer::after(Duration::from_millis(20)).await;
    pa5.set_high();
    let mut delay = Delay;
    delay.delay_us(30);
}

pub fn check_sensor_response(pa5: &mut Flex<'_>) -> Result<(), Dh11Error> {
    let low_pulse = match measure_pulse_width(pa5, false, 200) {
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
    let high_pulse = match measure_pulse_width(pa5, true, 200) {
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
fn measure_pulse_width(
    pin: &mut Flex<'_>,
    level_to_measure: bool,
    timeout_us: u64,
) -> Result<u64, Dh11Error> {
    wait_for_level(pin, level_to_measure, timeout_us)?;
    let start = Instant::now();
    wait_for_level(pin, !level_to_measure, timeout_us)?;
    Ok(start.elapsed().as_micros())
}
fn wait_for_level(pin: &mut Flex<'_>, target: bool, timeout_us: u64) -> Result<(), Dh11Error> {
    let start = Instant::now();
    loop {
        if pin.is_high() == target {
            return Ok(());
        }
        if start.elapsed().as_micros() > timeout_us {
            return Err(Dh11Error::TimeOut);
        }
    }
}

pub fn dh11_read(pin: &mut Flex<'_>) -> Result<[u8; 5], Dh11Error> {
    let mut bytes = [0u8; 5];
    for bit_index in 0..40 {
        if let Err(_) = wait_for_level(pin, true, 100) {
            error!("读取数据位 {} 时等待高电平超时", bit_index);
            return Err(Dh11Error::TimeOut);
        }
        let start = Instant::now();
        if let Err(_) = wait_for_level(pin, false, 100) {
            error!("读取数据位 {} 时等待低电平超时", bit_index);
            return Err(Dh11Error::TimeOut);
        }
        let pulse = start.elapsed().as_micros();
        let bit = if pulse > 30 { 1 } else { 0 };
        bytes[(bit_index / 8) as usize] <<= 1;
        bytes[(bit_index / 8) as usize] |= bit;
    }
    let sum: u16 = bytes[0] as u16 + bytes[1] as u16 + bytes[2] as u16 + bytes[3] as u16;
    if (sum & 0xFF) as u8 != bytes[4] {
        error!("校验和错误: 计算值={}, 实际值={}", sum & 0xFF, bytes[4]);
        return Err(Dh11Error::ChecksumError);
    }
    Ok(bytes)
}
