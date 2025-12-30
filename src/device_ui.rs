use crate::config::UI_CHANNEL;
use crate::protocol::{SensorData, TxMessage};
use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time; // Keep embassy_time for embassy_time::Delay
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use st7735_lcd::{Orientation, ST7735};

// UI 状态缓存
#[derive(Default)]
struct UiState {
    temp: Option<i16>,
    humid: Option<u16>,
    light: Option<u16>,
    soil: Option<u16>,
    fan: bool,
    pump: bool,
    light_act: bool,
    buzzer: bool,
}

#[task]
pub async fn ui_task(
    spi: Spi<'static, Async>,
    cs: Output<'static>,
    dc: Output<'static>,
    rst: Output<'static>,
) {
    let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut display = ST7735::new(spi_dev, dc, rst, true, false, 160, 128);

    // Initialize display
    let mut delay = embassy_time::Delay;
    display.init(&mut delay).unwrap();
    display.set_orientation(&Orientation::Landscape).unwrap();

    // 偏移量 (Column Offset, Row Offset)
    // 常见值: (0,0) Black Tab, (2,1) or (2,3) Green Tab
    display.set_offset(2, 3);

    display.clear(Rgb565::BLACK).unwrap();

    let style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();

    let mut state = UiState::default();
    let receiver = UI_CHANNEL.receiver();

    // Initial Draw (Labels)
    draw_labels(&mut display, &style);
    draw_actuators(&mut display, &style, &state);

    loop {
        // Wait for message
        let msg = receiver.receive().await;

        match msg {
            TxMessage::Sensor(data) => match data {
                SensorData::Temperature(v) => {
                    if state.temp != Some(v) {
                        state.temp = Some(v);
                        draw_temp(&mut display, &style, v);
                    }
                }
                SensorData::Humidity(v) => {
                    if state.humid != Some(v) {
                        state.humid = Some(v);
                        draw_humid(&mut display, &style, v);
                    }
                }
                SensorData::LightIntensity(v) => {
                    if state.light != Some(v) {
                        state.light = Some(v);
                        draw_light(&mut display, &style, v);
                    }
                }
                SensorData::SoilMoisture(v) => {
                    if state.soil != Some(v) {
                        state.soil = Some(v);
                        draw_soil(&mut display, &style, v);
                    }
                }
            },
            TxMessage::Actuator(status) => {
                match status.actuator {
                    crate::protocol::ActuatorTag::Fan => state.fan = status.state,
                    crate::protocol::ActuatorTag::Pump => state.pump = status.state,
                    crate::protocol::ActuatorTag::Light => state.light_act = status.state,
                    crate::protocol::ActuatorTag::Buzzer => state.buzzer = status.state,
                }
                draw_actuators(&mut display, &style, &state);
            }
            _ => {}
        }
    }
}

fn draw_labels<D>(display: &mut D, style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>)
where
    D: DrawTarget<Color = Rgb565>,
{
    Text::with_baseline(
        "Environment Monitor",
        Point::new(5, 5),
        *style,
        Baseline::Top,
    )
    .draw(display)
    .ok();

    // Compact Layout (Tight vertical spacing)
    Text::with_baseline("Temp:", Point::new(5, 20), *style, Baseline::Top)
        .draw(display)
        .ok();
    Text::with_baseline("Humid:", Point::new(5, 32), *style, Baseline::Top)
        .draw(display)
        .ok();
    Text::with_baseline("Light:", Point::new(5, 44), *style, Baseline::Top)
        .draw(display)
        .ok();
    Text::with_baseline("Soil:", Point::new(5, 56), *style, Baseline::Top)
        .draw(display)
        .ok();

    // Actuators Label
    Text::with_baseline("Actuators:", Point::new(5, 75), *style, Baseline::Top)
        .draw(display)
        .ok();
}

fn draw_temp<D>(
    display: &mut D,
    style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>,
    val: i16,
) where
    D: DrawTarget<Color = Rgb565>,
{
    // Display as x.xx C
    let whole = val / 100;
    let frac = val.abs() % 100;

    use core::fmt::Write;
    let mut s = heapless::String::<32>::new();
    write!(s, "{}.{:02} C   ", whole, frac).ok();

    Text::with_baseline(&s, Point::new(50, 20), *style, Baseline::Top)
        .draw(display)
        .ok();
}

fn draw_humid<D>(
    display: &mut D,
    style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>,
    val: u16,
) where
    D: DrawTarget<Color = Rgb565>,
{
    use core::fmt::Write;
    let whole = val / 100;
    let frac = val % 100;
    let mut s = heapless::String::<32>::new();
    write!(s, "{}.{:02} %   ", whole, frac).ok();
    Text::with_baseline(&s, Point::new(50, 32), *style, Baseline::Top)
        .draw(display)
        .ok();
}

fn draw_light<D>(
    display: &mut D,
    style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>,
    val: u16,
) where
    D: DrawTarget<Color = Rgb565>,
{
    use core::fmt::Write;
    let mut s = heapless::String::<32>::new();
    write!(s, "{} Lux   ", val).ok();
    Text::with_baseline(&s, Point::new(50, 44), *style, Baseline::Top)
        .draw(display)
        .ok();
}

fn draw_soil<D>(
    display: &mut D,
    style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>,
    val: u16,
) where
    D: DrawTarget<Color = Rgb565>,
{
    use core::fmt::Write;
    let mut s = heapless::String::<32>::new();
    write!(s, "{}   ", val).ok();
    Text::with_baseline(&s, Point::new(50, 56), *style, Baseline::Top)
        .draw(display)
        .ok();
}

fn draw_actuators<D>(
    display: &mut D,
    style: &embedded_graphics::mono_font::MonoTextStyle<Rgb565>,
    state: &UiState,
) where
    D: DrawTarget<Color = Rgb565>,
{
    use core::fmt::Write;
    let mut s = heapless::String::<64>::new();
    // Shortened names to save space
    // F:ON P:ON
    let f = if state.fan { "ON " } else { "OFF" };
    let p = if state.pump { "ON " } else { "OFF" };
    let l = if state.light_act { "ON " } else { "OFF" };
    let b = if state.buzzer { "ON " } else { "OFF" };

    // Line 1: Fan & Pump
    s.clear();
    write!(s, "Fan:{} Pmp:{}", f, p).ok();
    Text::with_baseline(&s, Point::new(5, 90), *style, Baseline::Top)
        .draw(display)
        .ok();

    // Line 2: Light & Buzzer
    s.clear();
    write!(s, "Lit:{} Buz:{}", l, b).ok();
    Text::with_baseline(&s, Point::new(5, 102), *style, Baseline::Top)
        .draw(display)
        .ok();
}
