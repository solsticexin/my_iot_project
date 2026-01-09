use core::str::from_utf8;
use defmt::warn;
use embassy_executor::task;
use embassy_stm32::mode::Async;
use embassy_stm32::usart::{UartRx, UartTx};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use heapless::String;



///循环发送frame，该任务只发送frame不做其他处理
#[task]
pub async fn uart_tx(mut tx:UartTx<'static,Async>,
                     tx_receiver:Receiver<'static,CriticalSectionRawMutex,String<128>,4>)
{
    loop {
        let frame= tx_receiver.receive().await;
        match tx.write(frame.as_bytes()).await {
            Ok(_)=>(),
            Err(e)=>{warn!("串口发送帧失败{}",e);continue;},
        }
    }
}
///该项目只接收数据，然后通过信道转到控制中心处理
#[task]
pub async fn uart_rx(mut rx:UartRx<'static,Async>,
                     rx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,2>)
{
    let mut buffer=[0u8,128];
    loop {
        let len=match rx.read_until_idle(&mut buffer).await {
            Ok(val)=>val,
            Err(e)=>{warn!("uart_rx 串口读取读取失败{}",e);continue;}
        };
        let frame=match from_utf8(&buffer[0..len]) {
            Ok(val)=>val,
            Err(_)=>{warn!("uart_rx读取数据转换utf8 str失败");continue;}
        };
        let frame=frame.parse::<String<128>>().unwrap();
        rx_sender.send(frame).await;
    }
}