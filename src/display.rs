use embassy_time::{Duration, Timer};
use heapless::{format, String};
use crate::st7735::ST7735;
use crate::control_center::SENSOR_COLLECT_DATA;
/// ST7735显示任务 (ST7735 Display Task)
///
/// 在屏幕上显示传感器数据标签和数值
/// (Display sensor labels and values on screen)
///
/// # 参数 (Parameters)
/// - `display`: ST7735显示驱动实例 (ST7735 display driver instance)
#[embassy_executor::task]
pub async fn display_task(mut display: ST7735<'static>) {
    // 初始化显示屏 (Initialize display)
    if let Err(_e) = display.init().await {
        // 初始化失败，退出任务 (Initialization failed, exit task)
        return;
    }

    // 定义显示参数 (Define display parameters)
    let label_x = 10u16; // 标签起始X坐标 (Label start X coordinate)
    let value_x = 50u16; // 数值起始X坐标 (Value start X coordinate, 10 + 48 + 10)
    let spacing = 1u16; // 数字字符间距 (Digit character spacing)
    let line_height = 20u16; // 行高 (Line height)

    let fg_color = ST7735::COLOR_WHITE; // 前景色：白色 (Foreground: white)
    let bg_color = ST7735::COLOR_BLACK; // 背景色：黑色 (Background: black)
    let value_color = ST7735::COLOR_YELLOW; // 数值颜色：黄色 (Value color: yellow)

    // 清空屏幕 (Clear screen)
    let _ = display.clear_screen(bg_color).await;
    // 显示温度 (Display temperature)
    let _ = draw_temperature(&mut display, label_x, 20, fg_color, bg_color).await;
    // 显示湿度 (Display humidity)
    let _ = draw_humidity(&mut display, label_x, 20 + line_height, fg_color, bg_color).await;
    // 显示土壤 (Display soil)
    let _ = draw_soil(&mut display, label_x, 20 + line_height * 2, fg_color, bg_color, ).await;
    // 显示光照 (Display light)
    let _ = draw_light(&mut display, label_x, 20 + line_height * 3, fg_color, bg_color, ).await;

    loop {
        //获取传感器收集数据
        let dht11_humi:String<16>=format!("{}",unsafe{SENSOR_COLLECT_DATA.humi}).unwrap();
        let dht11_temp:String<16>=format!("{}",unsafe{SENSOR_COLLECT_DATA.temp}).unwrap();
        let light:String<16>=format!("{:2}",unsafe{SENSOR_COLLECT_DATA.light}).unwrap();
        let soil:String<16>=format!("{}",unsafe{SENSOR_COLLECT_DATA.soil}).unwrap();
        //显示温度数据
        let _ = draw_digits(
            &mut display,
            value_x,
            20,
            dht11_temp.as_str(),
            spacing,
            value_color,
            bg_color,
        )
        .await;
        //此处一定到提前丢弃，不然会等待单次循环结束，才会丢弃。
        // 在cpu处理其他任务期间可能导致栈满了
        drop(dht11_temp);

        //显示湿度数据
        let _ = draw_digits(
            &mut display,
            value_x,
            20 + line_height,
            dht11_humi.as_str(),
            spacing,
            value_color,
            bg_color,
        )
        .await;
        //此处一定到提前丢弃，不然会等待单次循环结束，才会丢弃。
        // 在cpu处理其他任务期间可能导致栈满了
        drop(dht11_humi);

        //显示土壤湿度数据
        let _ = draw_digits(
            &mut display,
            value_x,
            20 + line_height * 2,
            soil.as_str(),
            spacing,
            value_color,
            bg_color,
        )
        .await;
        //此处一定到提前丢弃，不然会等待单次循环结束，才会丢弃。
        // 在cpu处理其他任务期间可能导致栈满了
        drop(soil);

        //绘制光照强度数据
        let _ = draw_digits(
            &mut display,
            value_x,
            20 + line_height * 3,
            light.as_str(),
            spacing,
            value_color,
            bg_color,
        )
        .await;
        //此处一定到提前丢弃，不然会等待单次循环结束，才会丢弃。
        // 在cpu处理其他任务期间可能导致栈满了
        drop(light);

        Timer::after(Duration::from_secs(3)).await;
    }
}

// ========================================================================
// 绘图辅助函数 (Drawing Helper Functions)
// ========================================================================

async fn draw_temperature(
    display: &mut ST7735<'_>,
    x: u16,
    y: u16,
    fg: u16,
    bg: u16,
) -> Result<(), embassy_stm32::spi::Error> {
    display
        .draw_chinese_char(x, y, &crate::font::CHAR_TEMPERATURE, fg, bg)
        .await
}

async fn draw_humidity(
    display: &mut ST7735<'_>,
    x: u16,
    y: u16,
    fg: u16,
    bg: u16,
) -> Result<(), embassy_stm32::spi::Error> {
    display
        .draw_chinese_char(x, y, &crate::font::CHAR_HUMIDITY, fg, bg)
        .await
}

async fn draw_soil(
    display: &mut ST7735<'_>,
    x: u16,
    y: u16,
    fg: u16,
    bg: u16,
) -> Result<(), embassy_stm32::spi::Error> {
    display
        .draw_chinese_char(x, y, &crate::font::CHAR_SOIL, fg, bg)
        .await
}

async fn draw_light(
    display: &mut ST7735<'_>,
    x: u16,
    y: u16,
    fg: u16,
    bg: u16,
) -> Result<(), embassy_stm32::spi::Error> {
    display
        .draw_chinese_char(x, y, &crate::font::CHAR_LIGHT, fg, bg)
        .await
}

async fn draw_digits(
    display: &mut ST7735<'_>,
    mut x: u16,
    y: u16,
    text: &str,
    spacing: u16,
    fg: u16,
    bg: u16,
) -> Result<(), embassy_stm32::spi::Error> {
    for ch in text.chars() {
        if let Some(bitmap) = crate::font::get_digit_bitmap(ch) {
            display.draw_ascii_char(x, y, bitmap, fg, bg).await?;
            x += 8 + spacing;
        }
    }
    Ok(())
}
