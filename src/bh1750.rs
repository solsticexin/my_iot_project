use crate::config;
use crate::fmt::info;
use embassy_stm32::i2c::I2c;
use embassy_stm32::i2c::Master;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn bh1750_read(i2c: &'static mut I2c<'static, Async, Master>) {
    // 初始化传感器 先向设备发起通电
    match i2c
        .write(config::BH1750_ADDR, &[config::CMD_POWER_ON])
        .await
    {
        Ok(_) => info!("BH1750通电！"),
        Err(e) => info!("IIC错误： {:?}", e),
    }
    if let Err(e) = i2c
        .write(config::BH1750_ADDR, &[config::CMD_H_RES_MODE])
        .await
    {
        info!("IIC模式失败：{:?}", e);
    }

    Timer::after(Duration::from_millis(180)).await;

    loop {
        let mut iic_buf: [u8; 2] = [0; 2];
        match i2c.read(config::BH1750_ADDR, &mut iic_buf).await {
            Ok(_) => {
                let raw_data: u16 = ((iic_buf[0] as u16) << 8) | (iic_buf[1] as u16);
                let lux: f32 = (raw_data as f32) / 1.2;
                info!("光照强度{}lux 原始数据{}", lux, raw_data);
            }
            Err(e) => info!("读取数据失败：{:?}", e),
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}
