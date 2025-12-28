use crate::esp01s::{self, CommandExecuteFrame};
use defmt::{info, warn};
use embassy_stm32::mode;
use embassy_stm32::usart::Uart;
#[embassy_executor::task]
pub async fn manually(uart: Uart<'static, mode::Async>, mut relay: esp01s::Relay<'static>) {
    let mut uart = esp01s::Esp01s::new(uart);
    loop {
        let mut buffer = [0u8; 128];
        let len = match uart.usart.read_until_idle(&mut buffer).await {
            Ok(len) => len,
            Err(_) => {
                warn!("串口读取数据失败！");
                continue;
            }
        };
        if len == 0 {
            info!("没有读到数据！")
        }
        // 只处理有数据的情况
        if len > 0 {
            // 将读取到的数据转换为字符串切片
            let data_str = match core::str::from_utf8(&buffer[..len]) {
                Ok(s) => s,
                Err(_) => {
                    // UTF-8解码错误，继续下一个循环
                    info!("UTF-8解码错误");
                    continue;
                }
            };
            // 反序列化为CommandExecuteFrame结构体
            match serde_json_core::from_str::<CommandExecuteFrame>(data_str) {
                Ok((command_frame, _)) => {
                    let command = esp01s::FrameType::CommandExecute(command_frame);
                    match uart.command_execute(command, &mut relay).await {
                        Err(_e) => {
                            warn!("FrameTypeError");
                            continue;
                        }
                        _ => (),
                    }
                }
                Err(_e) => {
                    warn!("反序列化失败");
                    continue;
                }
            }
        }
    }
}
