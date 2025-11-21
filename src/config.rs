use embassy_stm32::{rcc, time::mhz};
pub fn stm_config() -> embassy_stm32::Config {
    let mut stm_config = embassy_stm32::Config::default();
    let clocks_config = clocks_config();
    stm_config.rcc = clocks_config;
    stm_config
}
fn clocks_config() -> rcc::Config {
    let mut config = rcc::Config::new();
    config.hsi = true;
    config.hse = Some(rcc::Hse {
        freq: mhz(8),
        mode: rcc::HseMode::Oscillator,
    });
    config.sys = rcc::Sysclk::PLL1_P;
    config.pll = Some(rcc::Pll {
        src: rcc::PllSource::HSE,
        prediv: rcc::PllPreDiv::DIV1,
        mul: rcc::PllMul::MUL9,
    });
    config.ahb_pre = rcc::AHBPrescaler::DIV1;
    config.apb1_pre = rcc::APBPrescaler::DIV2;
    config.apb2_pre = rcc::APBPrescaler::DIV1;
    config.adc_pre = rcc::ADCPrescaler::DIV6;
    config.mux = rcc::mux::ClockMux::default();
    config.ls = rcc::LsConfig::default();
    config
}
