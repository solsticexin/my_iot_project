// use crate::fmt::error;
// use embassy_stm32::gpio::Output;
// use embassy_stm32::mode::Blocking;
// use embassy_stm32::spi::Spi;
// use embassy_time::Delay;
// use embedded_graphics::pixelcolor::Rgb565;
// use embedded_graphics::prelude::*;
// use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
// use st7735_lcd::{Orientation, ST7735};

// // 定义显示屏类型别名，方便在其他地方使用
// pub type St7735Display = ST7735<Spi<'static, Blocking>, Output<'static>, Output<'static>>;

// // 初始化屏幕函数
// pub fn init_screen(
//     spi: Spi<'static, Blocking>,
//     dc: Output<'static>,
//     rst: Output<'static>,
// ) -> St7735Display {
//     // 创建驱动实例 (128x160 分辨率)
//     let mut display = ST7735::new(spi, dc, rst, true, false, 128, 160);
//     let mut delay = Delay;
//     display.init(&mut delay).unwrap();

//     // 设置方向
//     match display.set_orientation(&Orientation::Portrait) {
//         Ok(_) => (),
//         Err(e) => {
//             error!("Failed to set orientation: {}", e);
//         }
//     };

//     // 设置偏移 (ST7735 常见偏移: 0,0 或 2,1 或 26,1，如果屏幕边缘有杂色条，调整这里)
//     display.set_offset(0, 0);

//     // 清屏 (黑色)
//     match display.clear(Rgb565::BLACK) {
//         Ok(_) => (),
//         Err(e) => {
//             error!("Failed to clear screen: {}", e);
//         }
//     };

//     // 画一个简单的边框
//     Rectangle::new(Point::new(0, 0), Size::new(128, 160))
//         .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 1))
//         .draw(&mut display)
//         .unwrap();

//     display
// }
