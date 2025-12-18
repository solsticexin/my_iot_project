use embassy_stm32::{gpio, gpio::Level, mode, spi};
use embassy_time::{Duration, Timer};
pub struct ST7735 {
    spi: spi::Spi<'static, mode::Async>,
    rst: gpio::Output<'static>,
    dc: gpio::Output<'static>,
    cs: gpio::Output<'static>,
}
impl ST7735 {
    pub fn new(
        spi: spi::Spi<'static, mode::Async>,
        rst: gpio::Output<'static>,
        dc: gpio::Output<'static>,
        cs: gpio::Output<'static>,
    ) -> Self {
        Self { spi, rst, dc, cs }
    }
    pub async fn init(&mut self) {
        // 1. 硬件复位
        self.rst.set_low();
        Timer::after(Duration::from_millis(20)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(150)).await;

        // 2. 软件复位
        self.write_command(0x01).await; // SWRESET
        Timer::after(Duration::from_millis(150)).await;

        // 3. 退出休眠
        self.write_command(0x11).await; // SLPOUT
        Timer::after(Duration::from_millis(150)).await;

        // 4. 帧率控制 (Frame Rate Control) - ⚠️ 这一步如果不加，屏幕可能会闪烁或无法初始化
        // 0xB1 (In normal mode)
        self.write_command(0xB1).await;
        self.write_data(&[0x01]).await; // RTNA
        self.write_data(&[0x2C]).await; // FPA
        self.write_data(&[0x2D]).await; // BPA

        // 5. 颜色模式 (16-bit RGB565)
        self.write_command(0x3A).await;
        self.write_data(&[0x05]).await;

        // 6. 扫描方向 (MADCTL) - 横屏设置
        self.write_command(0x36).await;
        // 0xA8: MY=1, MX=0, MV=1 (交换XY轴), ML=0, BGR=1
        // 效果: 横屏 (160x128), 接口朝左时字是正的
        self.write_data(&[0xA8]).await;

        // 7. 反显控制 (Inversion Control) - ⚠️ 如果发现黑色变成白色，请开启这一行
        // self.write_command(0x21).await; // INVON (反色开启)

        // 8. 开启显示
        self.write_command(0x29).await; // DISPON
        Timer::after(Duration::from_millis(100)).await;
    }
    pub async fn write_command(&mut self, command: u8) {
        self.dc.set_level(Level::Low);
        self.spi.write(&[command]).await.unwrap();
    }
    pub async fn write_data(&mut self, data: &[u8]) {
        self.dc.set_level(Level::High);
        self.spi.write(data).await.unwrap();
    }
}

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
