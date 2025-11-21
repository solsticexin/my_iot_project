#![no_std]
#![no_main]

mod config;
mod dh11;
mod fmt;
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Flex, Speed};
use embassy_time::{Duration, Timer};
use fmt::{error, info};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = config::stm_config();
    let p = embassy_stm32::init(config);
    let mut dh11_pin = Flex::new(p.PA5);
    dh11_pin.set_as_input_output(Speed::VeryHigh);

    loop {
        dh11::wake_up_sensor(&mut dh11_pin).await;
        match dh11::check_sensor_response(&mut dh11_pin) {
            Ok(_) => (),
            Err(_) => {
                Timer::after(Duration::from_secs(2)).await;
                continue;
            }
        };
        match dh11::dh11_read(&mut dh11_pin) {
            Ok(data) => {
                info!("dh11_read: {},{},{},{}", data[0], data[1], data[2], data[3]);
            }
            Err(e) => {
                error!("dh11_read error: {}", e);
            }
        };
        Timer::after(Duration::from_millis(2000)).await;
    }
}
