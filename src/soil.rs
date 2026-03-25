use defmt;
use embassy_stm32::{
    adc::Adc,
    peripherals::{ADC1, PA0},
};
use embassy_stm32::adc::SampleTime;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};

const MAX_SOIL:u32=4096;
const MIN_SOIL:u32=0;
#[embassy_executor::task]
pub async fn soil_task(
    mut adc: Adc<'static, ADC1>,
    mut pin: embassy_stm32::Peri<'static, PA0>,
    sender: Sender<'static,CriticalSectionRawMutex,u16,2>,
) {
    adc.set_sample_time(SampleTime::CYCLES239_5);
    loop {
        let value = adc.read(&mut pin).await;
        let percentage=value as u32 *100 /(MAX_SOIL-MIN_SOIL)/100;
        sender.send(percentage as u16).await;
        defmt::info!("Soil moisture: {}", value);
        Timer::after(Duration::from_secs(1)).await;
    }
}
