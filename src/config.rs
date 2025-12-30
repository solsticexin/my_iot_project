use crate::protocol::{ControlCommand, TxMessage};
use embassy_stm32::mode::Async;
use embassy_stm32::{bind_interrupts, peripherals, rcc, time::mhz, usart::UartTx};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
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
// 绑定中断
bind_interrupts!(pub struct Irqs {
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
    USART1  => embassy_stm32::usart::InterruptHandler<peripherals::USART1>;
    ADC1_2 => embassy_stm32::adc::InterruptHandler<peripherals::ADC1>;
});

//BH1750 常量
pub const BH1750_ADDR: u8 = 0x23; //接地时的地址
pub const CMD_POWER_ON: u8 = 0x01u8; //通电指令
pub const CMD_H_RES_MODE: u8 = 0x10; //连续高分辨率模式

//全局静态变量
pub static CHANNEL_DHT11: Channel<CriticalSectionRawMutex, [u8; 5], 2> = Channel::new();

//pub type SharedTx<'d> = Mutex<CriticalSectionRawMutex, UartTx<'d, Async>>;
pub static SHARED_TX: Mutex<CriticalSectionRawMutex, Option<UartTx<'static, Async>>> =
    Mutex::new(None);

pub static UART_TX_CHANNEL: Channel<CriticalSectionRawMutex, TxMessage, 8> = Channel::new();
pub static COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, ControlCommand, 4> = Channel::new();
