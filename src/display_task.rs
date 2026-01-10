use embassy_time::{Duration, Timer};

use crate::st7735::ST7735;

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
    let value_x = 68u16; // 数值起始X坐标 (Value start X coordinate, 10 + 48 + 10)
    let spacing = 2u16; // 数字字符间距 (Digit character spacing)
    let line_height = 20u16; // 行高 (Line height)

    let fg_color = ST7735::COLOR_WHITE; // 前景色：白色 (Foreground: white)
    let bg_color = ST7735::COLOR_BLACK; // 背景色：黑色 (Background: black)
    let value_color = ST7735::COLOR_YELLOW; // 数值颜色：黄色 (Value color: yellow)

    // 清空屏幕 (Clear screen)
    let _ = display.clear_screen(bg_color).await;

    loop {
        // 显示温度 (Display temperature)
        let _ = display
            .draw_temperature(label_x, 20, fg_color, bg_color)
            .await;
        let _ = display
            .draw_digits(value_x, 20, "333", spacing, value_color, bg_color)
            .await;

        // 显示湿度 (Display humidity)
        let _ = display
            .draw_humidity(label_x, 20 + line_height, fg_color, bg_color)
            .await;
        let _ = display
            .draw_digits(
                value_x,
                20 + line_height,
                "333",
                spacing,
                value_color,
                bg_color,
            )
            .await;

        // 显示土壤 (Display soil)
        let _ = display
            .draw_soil(label_x, 20 + line_height * 2, fg_color, bg_color)
            .await;
        let _ = display
            .draw_digits(
                value_x,
                20 + line_height * 2,
                "333",
                spacing,
                value_color,
                bg_color,
            )
            .await;

        // 显示光照 (Display light)
        let _ = display
            .draw_light(label_x, 20 + line_height * 3, fg_color, bg_color)
            .await;
        let _ = display
            .draw_digits(
                value_x,
                20 + line_height * 3,
                "333",
                spacing,
                value_color,
                bg_color,
            )
            .await;
        Timer::after(Duration::from_secs(3)).await;
    }
    // 任务完成后退出 (Exit after task completion)
    // 在实际应用中，这里可以添加循环来持续更新显示
    // (In real application, add a loop here to continuously update display)
}
