#![allow(dead_code)]

use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::Timer;

// ============================================================================
// ST7735 命令常量定义 (ST7735 Command Constants)
// ============================================================================

/// 软件复位命令 (Software Reset)
const CMD_SWRESET: u8 = 0x01;

/// 睡眠退出命令 (Sleep Out)
const CMD_SLPOUT: u8 = 0x11;

/// 显示反转关闭 (Display Inversion Off)
const CMD_INVOFF: u8 = 0x20;

/// 显示反转开启 (Display Inversion On)
const CMD_INVON: u8 = 0x21;

/// 伽马设置 (Gamma Set)
const CMD_GAMSET: u8 = 0x26;

/// 显示关闭 (Display Off)
const CMD_DISPOFF: u8 = 0x28;

/// 显示开启 (Display On)
const CMD_DISPON: u8 = 0x29;

/// 列地址设置 (Column Address Set)
const CMD_CASET: u8 = 0x2A;

/// 行地址设置 (Row Address Set)
const CMD_RASET: u8 = 0x2B;

/// 内存写入 (Memory Write)
const CMD_RAMWR: u8 = 0x2C;

/// 内存访问控制 (Memory Access Control)
const CMD_MADCTL: u8 = 0x36;

/// 像素格式设置 (Pixel Format Set)
const CMD_COLMOD: u8 = 0x3A;

/// 帧率控制(正常模式) (Frame Rate Control - Normal Mode)
const CMD_FRMCTR1: u8 = 0xB1;

/// 帧率控制(空闲模式) (Frame Rate Control - Idle Mode)
const CMD_FRMCTR2: u8 = 0xB2;

/// 帧率控制(部分模式) (Frame Rate Control - Partial Mode)
const CMD_FRMCTR3: u8 = 0xB3;

/// 显示反转控制 (Display Inversion Control)
const CMD_INVCTR: u8 = 0xB4;

/// 电源控制1 (Power Control 1)
const CMD_PWCTR1: u8 = 0xC0;

/// 电源控制2 (Power Control 2)
const CMD_PWCTR2: u8 = 0xC1;

/// 电源控制3 (Power Control 3)
const CMD_PWCTR3: u8 = 0xC2;

/// 电源控制4 (Power Control 4)
const CMD_PWCTR4: u8 = 0xC3;

/// 电源控制5 (Power Control 5)
const CMD_PWCTR5: u8 = 0xC4;

/// VCOM控制1 (VCOM Control 1)
const CMD_VMCTR1: u8 = 0xC5;

/// 正极伽马校正 (Positive Gamma Correction)
const CMD_GMCTRP1: u8 = 0xE0;

/// 负极伽马校正 (Negative Gamma Correction)
const CMD_GMCTRN1: u8 = 0xE1;

// ============================================================================
// ST7735 驱动结构体 (ST7735 Driver Structure)
// ============================================================================

/// ST7735 LCD 驱动结构体 (ST7735 LCD Driver Structure)
///
/// 支持异步SPI通信，包含必要的控制引脚
/// (Supports async SPI communication with necessary control pins)
pub struct ST7735<'d> {
    /// SPI设备接口 (SPI Device Interface)
    spi: Spi<'d, Async>,

    /// 复位引脚 (Reset Pin)
    res: Output<'d>,

    /// 数据/命令选择引脚 (Data/Command Select Pin)
    dc: Output<'d>,

    /// 片选引脚 (Chip Select Pin)
    cs: Output<'d>,

    /// 背光控制引脚(可选) (Backlight Control Pin - Optional)
    bl: Option<Output<'d>>,
}

impl<'d> ST7735<'d> {
    // ========================================================================
    // 构造函数 (Constructor)
    // ========================================================================

    /// 创建新的ST7735驱动实例 (Create new ST7735 driver instance)
    ///
    /// # 参数 (Parameters)
    /// - `spi`: 异步SPI设备接口 (Async SPI device interface)
    /// - `res`: 复位引脚 (Reset pin)
    /// - `dc`: 数据/命令选择引脚 (Data/Command select pin)
    /// - `cs`: 片选引脚 (Chip select pin)
    /// - `bl`: 背光控制引脚(可选) (Backlight control pin - optional)
    pub fn new(
        spi: Spi<'d, Async>,
        res: Output<'d>,
        dc: Output<'d>,
        cs: Output<'d>,
        bl: Option<Output<'d>>,
    ) -> Self {
        Self {
            spi,
            res,
            dc,
            cs,
            bl,
        }
    }

    // ========================================================================
    // 初始化函数 (Initialization Function)
    // ========================================================================

    /// 初始化ST7735显示屏 (Initialize ST7735 Display)
    ///
    /// 执行完整的初始化序列，包括复位、睡眠退出、显示配置等
    /// (Performs complete initialization sequence including reset, sleep out, display configuration, etc.)
    pub async fn init(&mut self) -> Result<(), embassy_stm32::spi::Error> {
        // 硬件复位 (Hardware Reset)
        self.reset().await;

        // 软件复位 (Software Reset)
        self.write_command(CMD_SWRESET).await?;
        Timer::after_millis(150).await;

        // 退出睡眠模式 (Exit Sleep Mode)
        self.write_command(CMD_SLPOUT).await?;
        Timer::after_millis(120).await;

        // 帧率控制 - 正常模式 (Frame Rate Control - Normal Mode)
        self.write_command(CMD_FRMCTR1).await?;
        self.write_data(&[0x01, 0x2C, 0x2D]).await?;

        // 帧率控制 - 空闲模式 (Frame Rate Control - Idle Mode)
        self.write_command(CMD_FRMCTR2).await?;
        self.write_data(&[0x01, 0x2C, 0x2D]).await?;

        // 帧率控制 - 部分模式 (Frame Rate Control - Partial Mode)
        self.write_command(CMD_FRMCTR3).await?;
        self.write_data(&[0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D])
            .await?;

        // 显示反转控制 (Display Inversion Control)
        self.write_command(CMD_INVCTR).await?;
        self.write_data(&[0x07]).await?;

        // 电源控制1 (Power Control 1)
        self.write_command(CMD_PWCTR1).await?;
        self.write_data(&[0xA2, 0x02, 0x84]).await?;

        // 电源控制2 (Power Control 2)
        self.write_command(CMD_PWCTR2).await?;
        self.write_data(&[0xC5]).await?;

        // 电源控制3 (Power Control 3)
        self.write_command(CMD_PWCTR3).await?;
        self.write_data(&[0x0A, 0x00]).await?;

        // 电源控制4 (Power Control 4)
        self.write_command(CMD_PWCTR4).await?;
        self.write_data(&[0x8A, 0x2A]).await?;

        // 电源控制5 (Power Control 5)
        self.write_command(CMD_PWCTR5).await?;
        self.write_data(&[0x8A, 0xEE]).await?;

        // VCOM控制1 (VCOM Control 1)
        self.write_command(CMD_VMCTR1).await?;
        self.write_data(&[0x0E]).await?;

        // 关闭显示反转 (Turn off Display Inversion)
        self.write_command(CMD_INVOFF).await?;

        // 内存访问控制 (Memory Access Control)
        self.write_command(CMD_MADCTL).await?;
        self.write_data(&[0xC8]).await?; // RGB, 行列交换 (RGB, row/column exchange)

        // 像素格式设置为16位RGB565 (Set Pixel Format to 16-bit RGB565)
        self.write_command(CMD_COLMOD).await?;
        self.write_data(&[0x05]).await?;

        // 正极伽马校正 (Positive Gamma Correction)
        self.write_command(CMD_GMCTRP1).await?;
        self.write_data(&[
            0x02, 0x1C, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2D, 0x29, 0x25, 0x2B, 0x39, 0x00, 0x01,
            0x03, 0x10,
        ])
        .await?;

        // 负极伽马校正 (Negative Gamma Correction)
        self.write_command(CMD_GMCTRN1).await?;
        self.write_data(&[
            0x03, 0x1D, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D, 0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00,
            0x02, 0x10,
        ])
        .await?;

        // 开启显示 (Turn on Display)
        self.write_command(CMD_DISPON).await?;
        Timer::after_millis(100).await;

        // 如果有背光引脚，开启背光 (Turn on backlight if available)
        if let Some(ref mut bl) = self.bl {
            let _ = bl.set_high();
        }

        Ok(())
    }

    // ========================================================================
    // 命令函数 (Command Function)
    // ========================================================================

    /// 向ST7735写入命令 (Write command to ST7735)
    ///
    /// # 参数 (Parameters)
    /// - `cmd`: 命令字节 (Command byte)
    pub async fn write_command(&mut self, cmd: u8) -> Result<(), embassy_stm32::spi::Error> {
        // 设置DC为低电平(命令模式) (Set DC low for command mode)
        let _ = self.dc.set_low();

        // 拉低CS选中设备 (Pull CS low to select device)
        let _ = self.cs.set_low();

        // 发送命令 (Send command)
        self.spi.write(&[cmd]).await?;

        // 拉高CS取消选中 (Pull CS high to deselect)
        let _ = self.cs.set_high();

        Ok(())
    }

    // ========================================================================
    // 数据函数 (Data Function)
    // ========================================================================

    /// 向ST7735写入数据 (Write data to ST7735)
    ///
    /// # 参数 (Parameters)
    /// - `data`: 数据字节数组 (Data byte array)
    pub async fn write_data(&mut self, data: &[u8]) -> Result<(), embassy_stm32::spi::Error> {
        // 设置DC为高电平(数据模式) (Set DC high for data mode)
        let _ = self.dc.set_high();

        // 拉低CS选中设备 (Pull CS low to select device)
        let _ = self.cs.set_low();

        // 发送数据 (Send data)
        self.spi.write(data).await?;

        // 拉高CS取消选中 (Pull CS high to deselect)
        let _ = self.cs.set_high();

        Ok(())
    }

    // ========================================================================
    // 辅助函数 (Helper Functions)
    // ========================================================================

    /// 硬件复位 (Hardware Reset)
    async fn reset(&mut self) {
        let _ = self.res.set_high();
        Timer::after_millis(10).await;
        let _ = self.res.set_low();
        Timer::after_millis(10).await;
        let _ = self.res.set_high();
        Timer::after_millis(10).await;
    }

    /// 设置显示窗口 (Set Display Window)
    ///
    /// # 参数 (Parameters)
    /// - `x0`: 起始X坐标 (Start X coordinate)
    /// - `y0`: 起始Y坐标 (Start Y coordinate)
    /// - `x1`: 结束X坐标 (End X coordinate)
    /// - `y1`: 结束Y坐标 (End Y coordinate)
    pub async fn set_window(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
    ) -> Result<(), embassy_stm32::spi::Error> {
        // 设置列地址 (Set Column Address)
        self.write_command(CMD_CASET).await?;
        self.write_data(&[
            (x0 >> 8) as u8,
            (x0 & 0xFF) as u8,
            (x1 >> 8) as u8,
            (x1 & 0xFF) as u8,
        ])
        .await?;

        // 设置行地址 (Set Row Address)
        self.write_command(CMD_RASET).await?;
        self.write_data(&[
            (y0 >> 8) as u8,
            (y0 & 0xFF) as u8,
            (y1 >> 8) as u8,
            (y1 & 0xFF) as u8,
        ])
        .await?;

        // 准备写入内存 (Prepare to write to memory)
        self.write_command(CMD_RAMWR).await?;

        Ok(())
    }

    /// 开启背光 (Turn on Backlight)
    pub fn backlight_on(&mut self) {
        if let Some(ref mut bl) = self.bl {
            let _ = bl.set_high();
        }
    }

    /// 关闭背光 (Turn off Backlight)
    pub fn backlight_off(&mut self) {
        if let Some(ref mut bl) = self.bl {
            let _ = bl.set_low();
        }
    }

    // ========================================================================
    // 颜色常量定义 (RGB565 Color Constants)
    // ========================================================================

    /// 黑色 (Black)
    pub const COLOR_BLACK: u16 = 0x0000;

    /// 白色 (White)
    pub const COLOR_WHITE: u16 = 0xFFFF;

    /// 红色 (Red)
    pub const COLOR_RED: u16 = 0xF800;

    /// 绿色 (Green)
    pub const COLOR_GREEN: u16 = 0x07E0;

    /// 蓝色 (Blue)
    pub const COLOR_BLUE: u16 = 0x001F;

    /// 黄色 (Yellow)
    pub const COLOR_YELLOW: u16 = 0xFFE0;

    /// 青色 (Cyan)
    pub const COLOR_CYAN: u16 = 0x07FF;

    /// 洋红色 (Magenta)
    pub const COLOR_MAGENTA: u16 = 0xF81F;

    /// 灰色 (Gray)
    pub const COLOR_GRAY: u16 = 0x8410;

    // ========================================================================
    // 屏幕控制函数 (Screen Control Functions)
    // ========================================================================

    /// 清空屏幕 (Clear screen)
    ///
    /// 用指定颜色填充整个屏幕
    /// (Fill the entire screen with specified color)
    ///
    /// # 参数 (Parameters)
    /// - `color`: 填充颜色 (Fill color)
    pub async fn clear_screen(&mut self, color: u16) -> Result<(), embassy_stm32::spi::Error> {
        // 设置窗口为整个屏幕 (Set window to entire screen)
        // ST7735有偏移，可以在此处设置。
        self.set_window(0, 0, 128, 160).await?;

        // 准备颜色数据 (Prepare color data)
        let color_bytes = [(color >> 8) as u8, (color & 0xFF) as u8];

        // 填充整个屏幕 (Fill entire screen)
        for _i in 0..(128 * 160) {
            self.write_data(&color_bytes).await?;
        }

        Ok(())
    }

    // ========================================================================
    // 字符显示函数 (Character Display Functions)
    // ========================================================================

    // ========================================================================
    // 内部辅助函数 (Internal Helper Functions) -> 公开函数
    // ========================================================================

    /// 绘制单个汉字字符 (Draw single Chinese character)
    /// 汉字为16x16，占据6列（48像素宽）
    pub async fn draw_chinese_char(
        &mut self,
        x: u16,
        y: u16,
        bitmap: &[[u8; 6]; 16],
        fg_color: u16,
        bg_color: u16,
    ) -> Result<(), embassy_stm32::spi::Error> {
        // 设置显示窗口 (Set display window)
        self.set_window(x, y, x + 47, y + 15).await?;

        // 准备像素缓冲区 (Prepare pixel buffer)
        let mut pixel_buffer = [0u8; 2]; // 每个像素2字节 (2 bytes per pixel)

        // 逐行逐列绘制 (Draw row by row, column by column)
        for row in 0..16 {
            for col_byte_idx in 0..6 {
                let byte = bitmap[row][col_byte_idx];

                // 每个字节包含8个像素 (Each byte contains 8 pixels)
                for bit in 0..8 {
                    let color = if (byte & (1 << (7 - bit))) != 0 {
                        fg_color
                    } else {
                        bg_color
                    };

                    // RGB565格式，高字节在前 (RGB565 format, high byte first)
                    pixel_buffer[0] = (color >> 8) as u8;
                    pixel_buffer[1] = (color & 0xFF) as u8;

                    self.write_data(&pixel_buffer).await?;
                }
            }
        }

        Ok(())
    }

    /// 绘制单个ASCII字符 (Draw single ASCII character)
    /// ASCII字符为16x16，占据2列（16像素宽）
    pub async fn draw_ascii_char(
        &mut self,
        x: u16,
        y: u16,
        bitmap: &[[u8; 2]; 16],
        fg_color: u16,
        bg_color: u16,
    ) -> Result<(), embassy_stm32::spi::Error> {
        // 设置显示窗口 (Set display window)
        self.set_window(x, y, x + 15, y + 15).await?;

        // 准备像素缓冲区 (Prepare pixel buffer)
        let mut pixel_buffer = [0u8; 2]; // 每个像素2字节 (2 bytes per pixel)

        // 逐行逐列绘制 (Draw row by row, column by column)
        for row in 0..16 {
            for col_byte_idx in 0..2 {
                let byte = bitmap[row][col_byte_idx];

                // 每个字节包含8个像素 (Each byte contains 8 pixels)
                for bit in 0..8 {
                    let color = if (byte & (1 << (7 - bit))) != 0 {
                        fg_color
                    } else {
                        bg_color
                    };

                    // RGB565格式，高字节在前 (RGB565 format, high byte first)
                    pixel_buffer[0] = (color >> 8) as u8;
                    pixel_buffer[1] = (color & 0xFF) as u8;

                    self.write_data(&pixel_buffer).await?;
                }
            }
        }

        Ok(())
    }
}
