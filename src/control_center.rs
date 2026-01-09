use defmt::warn;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_time::{with_timeout, Duration};
use heapless::{format, String};
use serde::Deserialize;
use crate::actuator::{Ack, Act, Actuator, ACTUATOR_STATUS};

// const DEFAULT_FRAME:&str="hello world\r\n";

#[task]
pub async fn control(
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,8>,
    dht11_receiver:Receiver<'static, CriticalSectionRawMutex, (u8,u8), 2>,
    bh1750_receiver:Receiver<'static, CriticalSectionRawMutex, f32, 2>,
    soil_receiver:Receiver<'static, CriticalSectionRawMutex, u16, 2>,
){
    loop {
        let dht11_data= with_timeout(
            Duration::from_millis(2500),
            dht11_receiver.receive()
        ).await.unwrap_or_else(|_| {
            warn!("dht11读取超时");
            (0, 0)
        });

        //湿度,温度解构不是临时参数
        let (humi,temp)=dht11_data;

        let bh1750_data= with_timeout(
            Duration::from_millis(2500),
            bh1750_receiver.receive()
        ).await.unwrap_or_else(|_| {
            warn!("bh1750读取超时");
            0.0
        });
        
        let soil_data= with_timeout(
            Duration::from_millis(2500),
            soil_receiver.receive()
        ).await.unwrap_or_else(|_| {
            warn!("soil读取超时");
            0
        });
        let water=unsafe{ACTUATOR_STATUS.water};
        let fan=unsafe{ACTUATOR_STATUS.fan};
        let light=unsafe{ACTUATOR_STATUS.light};
        let buzzer=unsafe{ACTUATOR_STATUS.buzzer};
        
        let frame: String<128> = format!(
            r#"{{"type":"data","temp":{},"humi":{},"soil":{},"lux":{},"water":{},"light":{},"fan":{},"buzzer":{}}}\r\n"#,
            temp as f32 ,
            humi as f32 ,
            soil_data,
            bh1750_data,
            water,
            light,
            fan,
            buzzer
        ).unwrap();
        
        tx_sender.send(frame).await;
    }
}

#[task]
pub async fn sub_control(
    rx_receiver:Receiver<'static,CriticalSectionRawMutex,String<128>,2>,
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,8>,
    mut actuator:Actuator<'static>,
){
    let rx_cmd_frame=rx_receiver.receive().await;
    let temp=rx_cmd_frame.as_str();
    let (cmd,_):(Cmd, usize)=serde_json_core::from_str(temp).unwrap();
    let cmd=cmd.analysis();
    let frame=ack_frame_wrap(&cmd);
    let (act,switch)=cmd.analysis();
   actuator.set(act,switch).await;
    tx_sender.send(frame).await;
}

#[derive(Deserialize, Debug, defmt::Format)]
struct Cmd {
    r#type: String<16>,
    target: String<16>,
    action: String<16>,
    time: u64,
}
impl Cmd {
    pub fn analysis(&self)->Ack{
        let target=self.target.as_str();
        let action=self.action.as_str();
        let time=self.time;
        let act=match target {
            "water"=>Act::Water,
            "fan" =>Act::Fan,
            "light" =>Act::Light,
            "buzzer" =>Act::Buzzer,
            _ => unreachable!()
        };
        match action {
            "on" => Ack::On(act),
            "off" =>Ack::Off(act),
            "pulse" => Ack::Pulse(act,time),
            _ => unreachable!()
        }
    }
}

pub fn ack_frame_wrap(ack:&Ack) ->String<128>{
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

