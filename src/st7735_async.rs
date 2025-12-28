// use crate::fmt::{error, info};
// use embassy_stm32::gpio::Output;
// use embassy_stm32::mode::Async;
// use embassy_stm32::spi::Spi;
// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
// use embassy_sync::channel::Receiver;
// use embassy_time::{Delay, Duration, Timer};
// use embedded_graphics::pixelcolor::Rgb565;
// use embedded_graphics::prelude::{RawData, RgbColor};
// use embedded_hal_bus::spi::ExclusiveDevice;
// use lcd_async::Builder;
// use lcd_async::interface::SpiInterface;
// use lcd_async::models::ST7735s;

// use static_cell::StaticCell;

// const WIDTH: usize = 128;
// const HEIGHT: usize = 160;
// // 使用行buffer策略，一次只缓存几行以节省RAM
// // 10行 * 128像素 * 2字节 = 2560字节
// const ROWS_PER_BUFFER: usize = 10;
// static LINE_BUFFER: StaticCell<[u8; WIDTH * ROWS_PER_BUFFER * 2]> = StaticCell::new();

// #[embassy_executor::task]
// pub async fn draw_task(
//     spi: Spi<'static, Async>,
//     dc: Output<'static>,
//     rst: Output<'static>,
//     cs: Output<'static>,
//     receiver: Receiver<'static, CriticalSectionRawMutex, [u8; 5], 2>,
// ) {
//     info!("Starting draw_task");

//     let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
//     let di = SpiInterface::new(spi_device, dc);

//     let mut display = match Builder::new(ST7735s, di)
//         .reset_pin(rst)
//         .init(&mut Delay)
//         .await
//     {
//         Ok(disp) => disp,
//         Err(_) => {
//             error!("Display init failed");
//             return;
//         }
//     };

//     // 初始化行buffer
//     let line_buf = LINE_BUFFER.init([0u8; WIDTH * ROWS_PER_BUFFER * 2]);

//     // 辅助函数：填充行buffer中的像素 (y是相对于buffer顶部的)
//     fn fill_line_buf(buf: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: Rgb565) {
//         let raw = embedded_graphics::pixelcolor::raw::RawU16::from(color).into_inner();
//         let high = (raw >> 8) as u8;
//         let low = (raw & 0xFF) as u8;

//         for py in y..(y + h) {
//             if py < ROWS_PER_BUFFER {
//                 for px in x..(x + w) {
//                     if px < WIDTH {
//                         let idx = (py * WIDTH + px) * 2;
//                         buf[idx] = high;
//                         buf[idx + 1] = low;
//                     }
//                 }
//             }
//         }
//     }

//     // 清屏
//     line_buf.fill(0);
//     // 分块清屏
//     for start_row in (0..HEIGHT).step_by(ROWS_PER_BUFFER) {
//         let rows = (HEIGHT - start_row).min(ROWS_PER_BUFFER);
//         let buf_size = WIDTH * rows * 2;
//         if let Err(_) = display
//             .show_raw_data(
//                 0,
//                 start_row as u16,
//                 (WIDTH - 1) as u16,
//                 (start_row + rows - 1) as u16,
//                 &line_buf[..buf_size],
//             )
//             .await
//         {
//             error!("Failed to clear screen");
//         }
//     }

//     loop {
//         let data = receiver.receive().await;
//         let hum_int = data[0];
//         let temp_int = data[2];

//         info!("Draw task received: hum={}, temp={}", hum_int, temp_int);

//         // 绘制温度条 (行30-40, 红色)
//         let temp_len = (temp_int as usize).min(100) * 2;
//         line_buf.fill(0); // 清空buffer
//         fill_line_buf(line_buf, 10, 0, temp_len, 10, Rgb565::RED);
//         if let Err(_) = display
//             .show_raw_data(0, 30, (WIDTH - 1) as u16, 39, &line_buf[..WIDTH * 10 * 2])
//             .await
//         {
//             error!("Failed to draw temp bar");
//         }

//         // 绘制湿度条 (行50-60, 青色)
//         let hum_len = (hum_int as usize).min(100);
//         line_buf.fill(0); // 清空buffer
//         fill_line_buf(line_buf, 10, 0, hum_len, 10, Rgb565::CYAN);
//         if let Err(_) = display
//             .show_raw_data(0, 50, (WIDTH - 1) as u16, 59, &line_buf[..WIDTH * 10 * 2])
//             .await
//         {
//             error!("Failed to draw hum bar");
//         }

//         Timer::after(Duration::from_secs(1)).await;
//     }
// }

//===============================
//draw任务
//===============================
// #[embassy_executor::task]
// async fn draw_task(
//     mut display: st7735::St7735Display,
//     receiver: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, [u8; 5], 2>,
// ) {
//     loop {
//         let data = receiver.receive().await;
//         let hum_int = data[0];
//         let temp_int = data[2];

//         // --- 可视化显示 (画条形图) ---

//         // 1. 清除旧的图形 (用黑色矩形覆盖)
//         Rectangle::new(Point::new(10, 20), Size::new(100, 60))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
//             .draw(&mut display)
//             .unwrap();

//         // 2. 画温度条 (红色) - 长度根据温度值变化
//         let temp_len = (temp_int as u32).min(100) * 2; // 放大一点便于观察
//         Rectangle::new(Point::new(10, 30), Size::new(temp_len, 10))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
//             .draw(&mut display)
//             .unwrap();

//         // 3. 画湿度条 (青色)
//         let hum_len = (hum_int as u32).min(100);
//         Rectangle::new(Point::new(10, 50), Size::new(hum_len, 10))
//             .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
//             .draw(&mut display)
//             .unwrap();
//         Timer::after(Duration::from_secs(2)).await
//     }
// }
