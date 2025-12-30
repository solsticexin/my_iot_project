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
pub async fn soil(mut adc: Adc<'static, ADC1>, pin: embassy_stm32::Peri<'static, PA0>) {
    use embassy_stm32::adc::SampleTime;
    let tx_sender = crate::config::UART_TX_CHANNEL.sender();
    adc.set_sample_time(SampleTime::CYCLES239_5);

    let mut soil = Soil::new(adc, pin);
    loop {
        defmt::info!("Starting soil read...");
        let value = soil.read().await;
        defmt::info!("Soil moisture: {}", value);

        let report =
            crate::protocol::TxMessage::Sensor(crate::protocol::SensorData::SoilMoisture(value));
        tx_sender.send(report).await;

        Timer::after(Duration::from_secs(1)).await;
    }
}
