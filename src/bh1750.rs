use crate::config;
use defmt;
use embassy_stm32::i2c::I2c;
use embassy_stm32::i2c::Master;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Timer};

pub type I2cDriver = I2c<'static, Async, Master>;

/// BH1750 光照传感器读取任务
/// 该任务获取 I2C 驱动的所有权，进行传感器初始化和周期性读取光照数据
#[embassy_executor::task]
pub async fn bh1750_read(mut i2c: I2cDriver) {
    let tx_sender = crate::config::UART_TX_CHANNEL.sender();
    defmt::info!("BH1750 任务已启动");

    // 初始化传感器：首先向设备发送通电命令
    match i2c
        .write(config::BH1750_ADDR, &[config::CMD_POWER_ON])
        .await
    {
        Ok(_) => defmt::info!("BH1750 通电成功！"),
        Err(e) => defmt::info!("IIC 错误： {:?}", e),
    }

    // 设置传感器为高分辨率模式 (H-Resolution Mode)
    if let Err(e) = i2c
        .write(config::BH1750_ADDR, &[config::CMD_H_RES_MODE])
        .await
    {
        defmt::info!("IIC 模式设置失败：{:?}", e);
    }

    // 等待传感器稳定 (高分辨率模式需要约 180ms)
    Timer::after(Duration::from_millis(180)).await;

    // 进入无限循环，每秒读取一次光照数据
    loop {
        // 准备缓冲区用于存储从传感器读取的 2 字节数据
        let mut iic_buf: [u8; 2] = [0; 2];

        // 从传感器读取光照数据
        match i2c.read(config::BH1750_ADDR, &mut iic_buf).await {
            Ok(_) => {
                // 将读取的字节数据转换为 16 位无符号整数
                let raw_data: u16 = ((iic_buf[0] as u16) << 8) | (iic_buf[1] as u16);
                // 根据 BH1750 数据手册，将原始数据转换为光照强度 (lux)
                // 高分辨率模式下，lux = raw_data / 1.2
                let lux: f32 = (raw_data as f32) / 1.2;
                defmt::info!("光照强度 {} lux，原始数据 {}", lux, raw_data);

                // 上报 (Cast raw data directly as protocol requests u16, or send calculated?)
                // API Document says LightIntensity (u16).
                // Let's send the raw/1.2 casted to u16.
                let lux_u16 = lux as u16;
                let report = crate::protocol::TxMessage::Sensor(
                    crate::protocol::SensorData::LightIntensity(lux_u16),
                );
                tx_sender.send(report).await;
            }
            Err(e) => defmt::info!("读取数据失败：{:?}", e),
        }

        // 等待 1 秒后进行下一次读取
        Timer::after(Duration::from_secs(1)).await;
    }
}
