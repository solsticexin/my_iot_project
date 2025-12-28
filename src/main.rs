#![no_std]
#![no_main]

mod bh1750;
mod config;
mod dht11;
mod esp01s;
mod esp01s_task;
mod fmt;
mod soil;
mod st7735;

use defmt::{error, info};
// mod st7735_async;
use crate::esp01s::Json;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::RgbColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;

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

#[embassy_executor::task]
async fn _test_usart_task(
    mut usart: embassy_stm32::usart::Uart<'static, embassy_stm32::mode::Async>,
) {
    // 使用 embassy_time::Timer 来实现延时
    use embassy_time::Timer;

    info!("=== ExecutionReceiptFrame 序列化测试启动 ===");
    info!("硬件连接检查清单:");
    info!("  [1] STM32 PA9  (TX) -> USB串口 RX");
    info!("  [3] GND -> GND");
    info!("  [4] 波特率: 115200, 8N1");
    info!("========================");

    loop {
        // 创建一个 ExecutionReceiptFrame 实例用于测试
        let mut receipt_frame = esp01s::ExecutionReceiptFrame {
            target: esp01s::Target::Water,
            action: esp01s::Action::On,
            result: true,
        };

        // 序列化 ExecutionReceiptFrame 为 JSON 字符串
        match receipt_frame.to_json::<128>() {
            Ok(mut json_str) => {
                // 添加换行符以便在串口终端中更好地显示
                if json_str.push_str("\r\n").is_ok() {
                    info!("序列化成功:");

                    // 通过 UART 发送 JSON 字符串
                    match usart.write(json_str.as_bytes()).await {
                        Ok(_) => info!("✓ 串口发送成功"),
                        Err(e) => error!("✗ 串口发送失败: {}", e),
                    }
                } else {
                    error!("✗ 添加换行符失败");
                }
            }
            Err(_) => {
                error!("✗ JSON 序列化失败:");
            }
        }

        // 每隔两秒执行一次测试
        Timer::after(Duration::from_secs(2)).await;
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
