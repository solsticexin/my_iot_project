use embassy_stm32::{gpio, gpio::Level, mode, spi};
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

#[derive(Clone, Copy, Debug)]
pub enum Orientation {
    Portrait,
    Landscape,
    PortraitSwapped,
    LandscapeSwapped,
}

/// ST7735 驱动结构体
///
/// 包含 SPI 接口、控制引脚以及屏幕状态信息。
pub struct ST7735 {
    /// SPI 接口 (Async)
    spi: spi::Spi<'static, mode::Async>,
    /// 复位引脚 (Reset)
    rst: gpio::Output<'static>,
    /// 数据/命令选择引脚 (Data/Command)
    dc: gpio::Output<'static>,
    /// 片选引脚 (Chip Select)
    cs: gpio::Output<'static>,
    // 内部状态
    /// 当前屏幕方向
    orientation: Orientation,
    /// 当前屏幕宽度
    width: u16,
    /// 当前屏幕高度
    height: u16,
    /// X轴偏移量 (用于校正不同批次屏幕的显示偏移)
    x_offset: u16,
    /// Y轴偏移量
    y_offset: u16,
}

impl ST7735 {
    // ST7735 指令常量定义
    /// 软件复位指令
    const SWRESET: u8 = 0x01;
    /// 退出休眠指令
    const SLPOUT: u8 = 0x11;
    /// 反显控制指令 (开启)
    const INVON: u8 = 0x21;
    /// 开启显示指令
    const DISPON: u8 = 0x29;
    /// 列地址设置指令
    const CASET: u8 = 0x2A;
    /// 行地址设置指令
    const RASET: u8 = 0x2B;
    /// 内存写入指令
    const RAMWR: u8 = 0x2C;
    /// 扫描方向控制指令
    const MADCTL: u8 = 0x36;
    /// 颜色模式设置指令
    const COLMOD: u8 = 0x3A;
    /// 帧率控制指令 (Normal Mode)
    const FRMCTR1: u8 = 0xB1;

    /// 创建一个新的 ST7735 驱动实例
    ///
    /// # 参数
    /// * `spi` - 配置好的异步 SPI 接口
    /// * `rst` - 复位引脚 Output
    /// * `dc` - 数据/命令选择引脚 Output
    /// * `cs` - 片选引脚 Output
    pub fn new(
        spi: spi::Spi<'static, mode::Async>,
        rst: gpio::Output<'static>,
        dc: gpio::Output<'static>,
        cs: gpio::Output<'static>,
    ) -> Self {
        Self {
            spi,
            rst,
            dc,
            cs,
            orientation: Orientation::Portrait, // 默认竖屏
            width: 128,
            height: 160,
            x_offset: 0,
            y_offset: 0,
        }
    }

    /// 初始化屏幕
    ///
    /// 执行标准的 ST7735 初始化序列：
    /// 1. 硬件复位
    /// 2. 软件复位
    /// 3. 退出休眠
    /// 4. 设置帧率、颜色模式
    /// 5. 设置扫描方向
    /// 6. 开启显示
    pub async fn init(&mut self) {
        // 1. 硬件复位
        self.rst.set_low();
        Timer::after(Duration::from_millis(20)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(150)).await;

        // 2. 软件复位
        self.write_command(Self::SWRESET).await;
        Timer::after(Duration::from_millis(150)).await;

        // 3. 退出休眠
        self.write_command(Self::SLPOUT).await;
        Timer::after(Duration::from_millis(150)).await;

        // 4. 帧率控制
        self.write_command(Self::FRMCTR1).await;
        self.write_data(&[0x01]).await; // RTNA
        self.write_data(&[0x2C]).await; // FPA
        self.write_data(&[0x2D]).await; // BPA

        // 5. 颜色模式 (16-bit RGB565)
        self.write_command(Self::COLMOD).await;
        self.write_data(&[0x05]).await;

        // 6. 扫描方向 (MADCTL)
        // 默认应用 Portrait 设置
        self.apply_orientation().await;

        // 7. 反显控制 (可选)
        // self.write_command(Self::INVON).await;

        // 8. 开启显示
        self.write_command(Self::DISPON).await;
        Timer::after(Duration::from_millis(100)).await;
    }

    /// 设置屏幕方向
    ///
    /// 支持 Portrait (竖屏), Landscape (横屏) 及其翻转模式。
    /// 调用此方法会自动更新屏幕的宽高信息和 MADCTL 寄存器。
    pub async fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
        self.apply_orientation().await;
    }

    /// 应用当前的方向设置到硬件
    async fn apply_orientation(&mut self) {
        let (madctl, w, h) = match self.orientation {
            Orientation::Portrait => (0x00, 128, 160),
            Orientation::Landscape => (0x60, 160, 128), // MV=1, MX=0, MY=1 (with offset fix usually) - 需根据实际微调
            Orientation::PortraitSwapped => (0xC0, 128, 160),
            Orientation::LandscapeSwapped => (0xA0, 160, 128),
        };
        // 注意：以上 MADCTL 值是常见 ST7735 配置，针对不同面板（红标/绿标/黑标）可能需要微调
        // ST7735S 通常为 BGR 顺序，所以通常需要 | 0x08 (BGR)
        // 让我们默认添加 BGR 位 (0x08)
        let madctl = madctl | 0x08;

        self.width = w;
        self.height = h;

        self.write_command(Self::MADCTL).await;
        self.write_data(&[madctl]).await;
    }

    /// 设置偏移 (针对不同屏幕边缘杂色校正)
    ///
    /// 许多 ST7735 屏幕的玻璃面板分辨率可能与驱动芯片默认设置不一致，导致显示内容偏移。
    /// 可以通过此函数手动修正起始坐标。
    pub fn set_offset(&mut self, x: u16, y: u16) {
        self.x_offset = x;
        self.y_offset = y;
    }

    /// 异步清屏
    ///
    /// 使用指定颜色填充整个屏幕。这是一个高效的操作。
    pub async fn clear(&mut self, color: Rgb565) {
        let w = self.width;
        let h = self.height;
        self.set_address_window(0, 0, w - 1, h - 1).await;

        // 颜色高低字节
        let color_u16 = color.into_storage();
        let high = (color_u16 >> 8) as u8;
        let low = (color_u16 & 0xFF) as u8;

        // 构造一个 buffer 行写入，提高效率
        // 假设最大 160 宽, 2 bytes/pixel
        let mut line_buf = [0u8; 320];
        let line_bytes = (w as usize) * 2;
        for i in 0..w as usize {
            line_buf[i * 2] = high;
            line_buf[i * 2 + 1] = low;
        }

        self.write_command(Self::RAMWR).await;
        self.dc.set_level(Level::High);

        for _ in 0..h {
            self.spi.write(&line_buf[..line_bytes]).await.unwrap();
        }
    }

    /// 设置绘制区域 (地址窗口)
    ///
    /// 定义接下来的数据写入操作将影响的屏幕区域。
    ///
    /// # 参数
    /// * `x0`, `y0` - 区域左上角坐标
    /// * `x1`, `y1` - 区域右下角坐标
    pub async fn set_address_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        let x0 = x0 + self.x_offset;
        let y0 = y0 + self.y_offset;
        let x1 = x1 + self.x_offset;
        let y1 = y1 + self.y_offset;

        // 设置列地址
        self.write_command(Self::CASET).await;
        self.write_data(&x0.to_be_bytes()).await;
        self.write_data(&x1.to_be_bytes()).await;

        // 设置行地址
        self.write_command(Self::RASET).await;
        self.write_data(&y0.to_be_bytes()).await;
        self.write_data(&y1.to_be_bytes()).await;

        // 准备写入数据
        self.write_command(Self::RAMWR).await;
    }

    /// 发送命令字节
    pub async fn write_command(&mut self, command: u8) {
        self.dc.set_level(Level::Low);
        self.spi.write(&[command]).await.unwrap();
    }

    /// 发送数据字节数组
    pub async fn write_data(&mut self, data: &[u8]) {
        self.dc.set_level(Level::High);
        self.spi.write(data).await.unwrap();
    }

    /// 异步绘制像素迭代器
    ///
    /// 由于 `embedded-graphics` 的 `DrawTarget` trait 是同步的，而此驱动使用了 async SPI，
    /// 因此我们提供这个辅助方法来绘制实现了 `IntoIterator<Item = Pixel<Rgb565>>` 的对象。
    /// 这允许我们在 async 上下文中绘制 `embedded-graphics` 的图元。
    pub async fn draw_pixels<I>(&mut self, pixels: I)
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(point, color) in pixels {
            // 检查边界
            if point.x >= 0
                && point.x < self.width as i32
                && point.y >= 0
                && point.y < self.height as i32
            {
                let x = point.x as u16;
                let y = point.y as u16;
                let color_u16 = color.into_storage();

                // 设置写入窗口为单个像素
                self.set_address_window(x, y, x, y).await;

                // 写入颜色数据 (2 bytes)
                self.dc.set_level(Level::High);
                self.spi.write(&color_u16.to_be_bytes()).await.unwrap();
            }
        }
    }
}

impl OriginDimensions for ST7735 {
    fn size(&self) -> Size {
        Size::new(self.width.into(), self.height.into())
    }
}
