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
mod st7735_display_task;

use crate::esp01s::Json;
use defmt::{error, info};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;

#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Flex, Speed},
    i2c::I2c,
    spi,
    time::khz,
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
    let adc = embassy_stm32::adc::Adc::new(p.ADC1, p.DMA1_CH1);

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
    let usart = match usart {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to create Uart: {}", e);
            return;
        }
    };

    // Relay Pins Configuration
    let mut relay_water = Flex::new(p.PB12);
    relay_water.set_as_output(Speed::Low);
    relay_water.set_low();

    let mut relay_light = Flex::new(p.PB13);
    relay_light.set_as_output(Speed::Low);
    relay_light.set_low();

    let mut relay_fan = Flex::new(p.PB14);
    relay_fan.set_as_output(Speed::Low);
    relay_fan.set_low();

    let mut relay_buzzer = Flex::new(p.PB15);
    relay_buzzer.set_as_output(Speed::Low);
    relay_buzzer.set_low();

    // Relay Instance
    let relay = esp01s::Relay::new(relay_fan, relay_buzzer, relay_water, relay_light);

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

    // Spawn ESP01s Hybrid Task
    match spawner.spawn(esp01s_task::hybrid(usart, relay)) {
        Ok(_) => {
            info!("ESP01s hybrid task started");
        }
        Err(e) => {
            error!("Failed to spawn hybrid task: {}", e);
        }
    }

    // ST7735 Display Task (Disabled)
    /*
    match spawner.spawn(st7735_display_task::display_task(display)) {
        Ok(_) => {
            info!("ST7735 display task started");
        }
        Err(e) => {
            error!("Failed to spawn display task: {}", e);
        }
    }
    */
}
