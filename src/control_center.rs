use defmt::{info, warn};
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_time::{with_timeout, Duration, Timer};
use heapless::{format, String};
use core::str::FromStr;
use serde::Deserialize;
use crate::actuator::{Ack, Act};
const DEFAULT_FRAME:&str="hello world\r\n";
#[task]
pub async fn control(
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,2>,
    rx_receiver:Receiver<'static,CriticalSectionRawMutex,String<128>,2>
){
    loop {
        let rx_frame=match with_timeout(
            Duration::from_secs(1),
            rx_receiver.receive()).await{
            Ok(val)=>val,
            Err(_)=>{warn!("rx_receiver接收超时");String::<128>::from_str(DEFAULT_FRAME).unwrap()}
        };
         tx_sender.send(rx_frame).await;
        Timer::after(Duration::from_millis(500)).await;
        let json_str = r#"{"type":"cmd","target":"light","action":"on","time":500}"#;
        let json_frame:String<128>=format!("{}\r\n",json_str).unwrap();
        let json_frame=json_frame.as_str();
        let (cmd, _): (Cmd, usize) = serde_json_core::from_str(json_frame).unwrap();
        info!("{}",cmd);
    }
}

#[derive(Deserialize, Debug, defmt::Format)]
struct Cmd {
    r#type: String<16>,
    target: String<16>,
    action: String<16>,
    time: u32,
}
pub fn ack_frame_wrap(ack:Ack) ->String<64>{
    match ack {
        Ack::On(act)=>{match act {
            Act::Light=>{
                let temp=r#"{"type":"ack","target":"light","action":"on","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Water=>{
                let temp=r#"{"type":"ack","target":"water","action":"on","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Fan=>{
                let temp=r#"{"type":"ack","target":"fan","action":"on","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Buzzer=>{
                let temp=r#"{"type":"ack","target":"buzzer","action":"on","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
        }}
        Ack::Off(act)=>{match act {
            Act::Light=>{
                let temp=r#"{"type":"ack","target":"light","action":"off","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Water=>{
                let temp=r#"{"type":"ack","target":"water","action":"off","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Fan=>{
                let temp=r#"{"type":"ack","target":"fan","action":"off","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Buzzer=>{
                let temp=r#"{"type":"ack","target":"buzzer","action":"off","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
        }}
        Ack::Pulse(act,_time)=>{match act {
            Act::Light=>{
                let temp=r#"{"type":"ack","target":"light","action":"pulse","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Water=>{
                let temp=r#"{"type":"ack","target":"water","action":"pulse","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Fan=>{
                let temp=r#"{"type":"ack","target":"fan","action":"pulse","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
            Act::Buzzer=>{
                let temp=r#"{"type":"ack","target":"buzzer","action":"pulse","result":"ok"}"#;
                format!("{}\r\n",temp).unwrap()
            }
        }}
    }
}