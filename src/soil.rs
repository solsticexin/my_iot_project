use defmt;
use embassy_stm32::{
    adc::{Adc, AdcChannel, Instance},
    peripherals::{ADC1, PA0},
};
use embassy_time::{Duration, Timer};

pub struct Soil<'d, T: Instance, P: AdcChannel<T>> {
    adc: Adc<'d, T>,
    pin: P,
}
impl<'d, T: Instance, P: AdcChannel<T>> Soil<'d, T, P> {
    pub fn new(adc: Adc<'d, T>, pin: P) -> Self {
        Self { adc, pin }
    }
    pub async fn read(&mut self) -> u16 {
        self.adc.read(&mut self.pin).await
    }
}
#[embassy_executor::task]
pub async fn soil(adc: Adc<'static, ADC1>, pin: embassy_stm32::Peri<'static, PA0>) {
    let mut soil = Soil::new(adc, pin);
    loop {
        let value = soil.read().await;
        defmt::info!("Soil moisture: {}", value);
        Timer::after(Duration::from_secs(1)).await;
    }
}
