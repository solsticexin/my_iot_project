#![no_std]
#![no_main]

mod bh1750;
mod config;
mod dht11;
mod fmt;
mod soil;
mod st7735;
// mod st7735_async;
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

use fmt::error;

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
    //串口
    let  usart1_config=embassy_stm32::usart::Config::default();
    let uart=embassy_stm32::usart::Uart::new(
        p.USART1,
        p.PA10,
        p.PA9,
        config::Irqs,
        p.DMA1_CH4,
        p.DMA1_CH5,
        usart1_config,
    );
    //===============================
    //执行dh11任务
    match spawner.spawn(dht11::dh11_task(dh11_pin, sender)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn task: {}", e);
        }
    }
    match spawner.spawn(test_st7735_task(spi_async, dc, rst, cs)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn test_st7735_task: {}", e);
        }
    }
    match spawner.spawn(bh1750::bh1750_read(i2c_bh1750)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn bh1750_read task: {}", e);
        }
    }
    match spawner.spawn(soil::soil(adc, p.PA0)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn soil task: {}", e);
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
