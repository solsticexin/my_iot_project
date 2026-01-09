use core::str::FromStr;
use defmt::warn;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_time::{with_timeout, Duration};
use heapless::String;

const MOREN:&str="hello world\r\n";
#[task]
pub async fn control(
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,2>,
    rx_receiver:Receiver<'static,CriticalSectionRawMutex,String<128>,2>
){
    loop {
        let rx_receive=match with_timeout(
            Duration::from_secs(1),
            rx_receiver.receive().await){
            Ok(val)=>val,
            Err(_)=>{warn!("rx_receiver接收超时");String::from_str(MOREN).unwrap()}
        };
         tx_sender.send(rx_receive).await;
    }
}