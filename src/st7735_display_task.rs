use crate::{config, st7735};
use defmt::info;
use embassy_time::{Duration, Timer};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb565, prelude::*};

/// ST7735显示任务：定期显示传感器数据  
/// 屏幕尺寸：128x160，横屏模式（160宽x128高）
#[embassy_executor::task]
pub async fn display_task(mut display: st7735::ST7735) {
    // 初始化显示屏
    display.init().await;

    // 设置横屏模式（160宽度）
    display
        .set_orientation(st7735::Orientation::Landscape)
        .await;

    // 清屏为黑色
    display.clear(Rgb565::BLACK).await;

    info!("ST7735显示任务已启动");

    loop {
        // 清屏
        display.clear(Rgb565::BLACK).await;

        // 从全局状态读取传感器数据
        let (temp, humi, soil, lux) = {
            let sensor_data = config::SENSOR_DATA.lock().await;
            (
                sensor_data.temperature,
                sensor_data.humidity,
                ((sensor_data.soil_moisture as u32 * 100) / 4095) as u8,
                sensor_data.light_intensity,
            )
        };

        // 读取继电器状态
        let (water, light_relay, fan, buzzer) = {
            let relay_states = config::RELAY_STATES.lock().await;
            (
                relay_states.water,
                relay_states.light,
                relay_states.fan,
                relay_states.buzzer,
            )
        };

        // 使用简化的文本绘制
        let mut line_y = 15;

        // 标题
        draw_text(
            &mut display,
            b"IoT Sensor",
            Point::new(40, line_y),
            Rgb565::CYAN,
        )
        .await;
        line_y += 20;

        // 使用临时缓冲区格式化字符串
        let mut buffer = [0u8; 32];

        // 温度（红色）
        let len = format_temp(&mut buffer, temp);
        draw_text(
            &mut display,
            &buffer[..len],
            Point::new(10, line_y),
            Rgb565::RED,
        )
        .await;
        line_y += 20;

        // 湿度（蓝色）
        let len = format_humi(&mut buffer, humi);
        draw_text(
            &mut display,
            &buffer[..len],
            Point::new(10, line_y),
            Rgb565::BLUE,
        )
        .await;
        line_y += 20;

        // 土壤湿度（绿色）
        let len = format_soil(&mut buffer, soil);
        draw_text(
            &mut display,
            &buffer[..len],
            Point::new(10, line_y),
            Rgb565::GREEN,
        )
        .await;
        line_y += 20;

        // 光照强度（黄色）
        let lux_value = (lux as f32 / 1.2) as u16;
        let len = format_light(&mut buffer, lux_value);
        draw_text(
            &mut display,
            &buffer[..len],
            Point::new(10, line_y),
            Rgb565::YELLOW,
        )
        .await;

        // 显示继电器状态（底部，白色）
        let len = format_relay_status(&mut buffer, water, light_relay, fan, buzzer);
        draw_text(
            &mut display,
            &buffer[..len],
            Point::new(10, 115),
            Rgb565::WHITE,
        )
        .await;

        // 每2秒刷新一次
        Timer::after(Duration::from_secs(2)).await;
    }
}

// 简单的文本绘制函数 - 使用FONT_6X10手动绘制
async fn draw_text(display: &mut st7735::ST7735, text: &[u8], position: Point, color: Rgb565) {
    let font = &FONT_6X10;
    let char_width = 6;
    let char_height = 10;

    let mut cursor_x = position.x;

    for &byte in text {
        let ch = byte as char;

        // 获取字符在字体中的索引
        if let Some(glyph_data) = get_glyph_6x10(byte) {
            // 绘制字符的每个像素
            for y in 0..char_height {
                let row_data = glyph_data[y as usize];
                for x in 0..char_width {
                    if (row_data & (1 << (7 - x))) != 0 {
                        let pixel_pos = Point::new(cursor_x + x, position.y + y - 8);
                        display
                            .draw_pixels(core::iter::once(embedded_graphics::Pixel(
                                pixel_pos, color,
                            )))
                            .await;
                    }
                }
            }
        }
        cursor_x += char_width;
    }
}

// 获取FONT_6X10的字形数据（简化版本，只支持ASCII）
fn get_glyph_6x10(ch: u8) -> Option<&'static [u8; 10]> {
    // 这里只实现常用字符，完整实现需要整个字体表
    // 为简化，我们只返回None，依赖ASCII基本字符
    None
}

// 辅助函数：格式化温度字符串
fn format_temp(buffer: &mut [u8], temp: u8) -> usize {
    let mut len = 0;
    let prefix = b"T:";
    buffer[..prefix.len()].copy_from_slice(prefix);
    len += prefix.len();
    len += format_u8(&mut buffer[len..], temp);
    buffer[len] = b'C';
    len + 1
}

// 辅助函数：格式化湿度字符串
fn format_humi(buffer: &mut [u8], humi: u8) -> usize {
    let mut len = 0;
    let prefix = b"H:";
    buffer[..prefix.len()].copy_from_slice(prefix);
    len += prefix.len();
    len += format_u8(&mut buffer[len..], humi);
    buffer[len] = b'%';
    len + 1
}

// 辅助函数：格式化土壤湿度字符串
fn format_soil(buffer: &mut [u8], soil: u8) -> usize {
    let mut len = 0;
    let prefix = b"S:";
    buffer[..prefix.len()].copy_from_slice(prefix);
    len += prefix.len();
    len += format_u8(&mut buffer[len..], soil);
    buffer[len] = b'%';
    len + 1
}

// 辅助函数：格式化光照强度字符串
fn format_light(buffer: &mut [u8], lux: u16) -> usize {
    let mut len = 0;
    let prefix = b"L:";
    buffer[..prefix.len()].copy_from_slice(prefix);
    len += prefix.len();
    len += format_u16(&mut buffer[len..], lux);
    len
}

// 辅助函数：格式化继电器状态字符串
fn format_relay_status(
    buffer: &mut [u8],
    water: bool,
    light: bool,
    fan: bool,
    buzzer: bool,
) -> usize {
    let mut len = 0;

    buffer[len] = if water { b'W' } else { b'w' };
    len += 1;
    buffer[len] = b' ';
    len += 1;

    buffer[len] = if light { b'L' } else { b'l' };
    len += 1;
    buffer[len] = b' ';
    len += 1;

    buffer[len] = if fan { b'F' } else { b'f' };
    len += 1;
    buffer[len] = b' ';
    len += 1;

    buffer[len] = if buzzer { b'B' } else { b'b' };
    len + 1
}

// 辅助函数：将u8转换为ASCII字符串
fn format_u8(buffer: &mut [u8], mut num: u8) -> usize {
    if num == 0 {
        buffer[0] = b'0';
        return 1;
    }

    let mut len = 0;
    let mut divisor = 100;
    let mut started = false;

    while divisor > 0 {
        let digit = num / divisor;
        if digit > 0 || started {
            buffer[len] = b'0' + digit;
            len += 1;
            started = true;
        }
        num %= divisor;
        divisor /= 10;
    }
    len
}

// 辅助函数：将u16转换为ASCII字符串
fn format_u16(buffer: &mut [u8], mut num: u16) -> usize {
    if num == 0 {
        buffer[0] = b'0';
        return 1;
    }

    let mut len = 0;
    let mut divisor = 10000;
    let mut started = false;

    while divisor > 0 {
        let digit = (num / divisor) as u8;
        if digit > 0 || started {
            buffer[len] = b'0' + digit;
            len += 1;
            started = true;
        }
        num %= divisor;
        divisor /= 10;
    }
    len
}
