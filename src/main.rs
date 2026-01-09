#![no_std]
#![no_main]

mod bh1750;
mod config;
mod dht11;
mod fmt;
mod soil;
mod uart;
mod control_center;
mod actuator;

use defmt::info;
// use embedded_graphics::{
//     pixelcolor::Rgb565,
//     prelude::RgbColor,
//     prelude::*,
//     primitives::{Circle, PrimitiveStyle},
// };
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
    let dht11_sender = config::DHT11_CHANNEL.sender();
    let _dht11_receiver = config::DHT11_CHANNEL.receiver();

    let bh1750_sender=config::BH1750_CHANNEL.sender();
    let bh1750_receiver=config::BH1750_CHANNEL.receiver();

    let soil_sender=config::SOIL_CHANNEL.sender();
    let soil_receiver=config::SOIL_CHANNEL.receiver();

    let tx_sender=config::TX_CHANNEL.sender();
    let tx_receiver=config::TX_CHANNEL.receiver();

    let rx_sender=config::RX_CHANNEL.sender();
    let rx_receiver=config::RX_CHANNEL.receiver();
    //===============================
    //配置dh11
    //===============================
    let mut dh11_pin = Flex::new(p.PA1);
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
    let mut _usart_config =embassy_stm32::usart::Config::default();
    _usart_config.baudrate=115200;//设置波特率
    _usart_config.data_bits=embassy_stm32::usart::DataBits::DataBits8;//设置数据位为8位
    _usart_config.stop_bits=embassy_stm32::usart::StopBits::STOP1;//设置停止位为1位
    _usart_config.parity=embassy_stm32::usart::Parity::ParityNone;//设置无校验位
    let usart=embassy_stm32::usart::Uart::new(
        p.USART1,
        p.PA10,
        p.PA9,
        config::Irqs,
        p.DMA1_CH4,
        p.DMA1_CH5,
        _usart_config,
    );
    let usart = match usart {
        Ok(val)=>val,
        Err(e)=>{
            error!("Failed to create Uart: {}", e);
            return;
        },
    };
    let (tx,rx)=usart.split();
    //===============================
    //执行任务
    match spawner.spawn(control_center::control(tx_sender,rx_receiver)) {
        Ok(_)=>(),
        Err(e)=>{
            error!("Failed to spawn dh11_task task: {}", e);
        }
    }
    match spawner.spawn(dht11::dh11_task(dh11_pin, dht11_sender)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn dh11_task task: {}", e);
        }
    }

    match spawner.spawn(bh1750::bh1750_read(i2c_bh1750,bh1750_sender)) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to spawn bh1750_read task: {}", e);
        }
    }

    match spawner.spawn(soil::soil_task(adc,p.PA0,soil_sender)) {
        Ok(_)=>(),
        Err(e)=>{
            error!("Failed to spawn soil_task task: {}", e)
        }
    }

    match spawner.spawn(uart::uart_tx(tx,tx_receiver)) {
        Ok(_)=>(),
        Err(e)=>{
            error!("Failed to spawn uart_tx task: {}", e)
        }
    }
    match spawner.spawn(uart::uart_rx(rx,rx_sender)) {
        Ok(_)=>(),
        Err(e)=>{
            error!("Failed to spawn uart_rx task: {}", e)
        }
    }
    //===============================
}