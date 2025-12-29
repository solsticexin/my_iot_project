#![no_std]
#![no_main]

mod bh1750;
mod config;
mod dht11;
mod esp01s;
mod fmt;
mod soil;

use defmt::{error, info, warn};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;

#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use config::SHARED_TX;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Flex, Level, Output, Speed},
    i2c::I2c,
    mode,
    spi::{self, Spi},
    time::{khz, mhz},
    usart,
};
use embassy_time::{Duration, Timer};
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialization
    let config = config::stm_config();
    let p = embassy_stm32::init(config);
    let sender = config::CHANNEL.sender();
    let _receiver = config::CHANNEL.receiver();

    // DHT11 Configuration (PA1)
    let mut dh11_pin = Flex::new(p.PA1);
    dh11_pin.set_as_input_output(Speed::VeryHigh);

    // ST7735 Configuration (Disabled)
    /*
    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(15);
    let spi_async = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA1_CH3, p.DMA1_CH2, spi_config,
    );
    let cs = Output::new(p.PA3, Level::Low, Speed::VeryHigh);
    let dc = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    let rst = Output::new(p.PA2, Level::Low, Speed::VeryHigh);
    let display = st7735::ST7735::new(spi_async, rst, dc, cs);
    */

    // I2C BH1750 Configuration
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

    // ADC for Soil Sensor
    let adc = embassy_stm32::adc::Adc::new(p.ADC1);

    // USART Configuration
    let mut _usart1_config = embassy_stm32::usart::Config::default();
    _usart1_config.baudrate = 115200;
    _usart1_config.data_bits = embassy_stm32::usart::DataBits::DataBits8;
    _usart1_config.stop_bits = embassy_stm32::usart::StopBits::STOP1;
    _usart1_config.parity = embassy_stm32::usart::Parity::ParityNone;
    let usart = embassy_stm32::usart::Uart::new(
        p.USART1,
        p.PA10,
        p.PA9,
        config::Irqs,
        p.DMA1_CH4,
        p.DMA1_CH5,
        _usart1_config,
    );
    let mut usart = match usart {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to create Uart: {}", e);
            return;
        }
    };

    usart.write(b"hello world\r\n").await.unwrap();
    let (tx, rx) = usart.split();
    // 设置共享TX端和RX端
    *SHARED_TX.lock().await = Some(tx);
    // Spawn DHT11 Task
    match spawner.spawn(dht11::dh11_task(dh11_pin, sender)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn task: {}", e);
        }
    }

    // Spawn BH1750 Task
    match spawner.spawn(bh1750::bh1750_read(i2c_bh1750)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn bh1750_read task: {}", e);
        }
    }

    // Spawn Soil Task
    match spawner.spawn(soil::soil(adc, p.PA0)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn soil task: {}", e);
        }
    }

    // Spawn Test Task 1 (Send Serial Data)
    match spawner.spawn(test_task1()) {
        Ok(_) => info!("Spawned test_task1"),
        Err(e) => error!("Failed to spawn test_task1: {}", e),
    }

    // Spawn Test Task 2 (Receive and Echo Serial Data)
    match spawner.spawn(test_task2(rx)) {
        Ok(_) => info!("Spawned test_task2"),
        Err(e) => error!("Failed to spawn test_task2: {}", e),
    }
}
#[embassy_executor::task]
async fn test_task1() {
    loop {
        let mut tx_g = SHARED_TX.lock().await;
        let tx = match tx_g.as_mut() {
            Some(tx) => tx,
            None => {
                warn!("task1 TX失败");
                continue;
            }
        };
        match tx.write(b"hello wolrd\r\n").await {
            Ok(_) => (),
            Err(e) => warn!("{}", e),
        }
        drop(tx_g);
        // 等待1秒
        Timer::after(Duration::from_secs(1)).await;
    }
}
#[embassy_executor::task]
async fn test_task2(mut rx: usart::UartRx<'static, mode::Async>) {
    let mut buffer = [0u8; 128];
    loop {
        // 使用embassy_time::with_timeout实现1秒超时
        let len = match embassy_time::with_timeout(
            Duration::from_secs(1),
            rx.read_until_idle(&mut buffer),
        )
        .await
        {
            Ok(Ok(val)) => {
                info!("task2 串口接收成功");
                val
            } // 成功读取到数据
            Ok(Err(e)) => {
                // 读取操作本身失败
                warn!("rx读取长度失败{}", e);
                continue;
            }
            Err(_) => {
                // 超时错误
                warn!("task2 串口接收超时");
                continue;
            }
        };
        if len > 0 {
            let mut tx_g = SHARED_TX.lock().await;
            let tx = match tx_g.as_mut() {
                Some(val) => val,
                None => {
                    warn!("task2 TX失败");
                    continue;
                }
            };
            match tx.write(&mut buffer[..len]).await {
                Ok(_) => (),
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            }
            drop(tx_g);
        }
    }
}
