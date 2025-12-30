use defmt;
use embassy_stm32::{
    adc::Adc,
    peripherals::{ADC1, PA0},
};
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn soil(mut adc: Adc<'static, ADC1>, mut pin: embassy_stm32::Peri<'static, PA0>) {
    use embassy_stm32::adc::SampleTime;
    let tx_sender = crate::config::UART_TX_CHANNEL.sender();
    adc.set_sample_time(SampleTime::CYCLES239_5);

    let ui_sender = crate::config::UI_CHANNEL.sender();

    loop {
        defmt::info!("Starting soil read...");
        let mut v = adc.read(&mut pin).await;

        // 翻转值, 越湿越小（0-4095） -> 越湿越大
        v = 4095 - v;

        defmt::info!("Soil moisture: {}", v);

        // API 定义 SoilMoisture 为 u16
        let report =
            crate::protocol::TxMessage::Sensor(crate::protocol::SensorData::SoilMoisture(v));
        tx_sender.send(report.clone()).await;
        let _ = ui_sender.try_send(report);

        Timer::after(Duration::from_secs(1)).await;
    }
}
