use defmt::warn;
use embassy_executor::task;
use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use heapless::{format, String};
use serde::Deserialize;
use crate::actuator::{Ack, Act, Actuator,Switch, ACTUATOR_STATUS};


pub struct DataCollect{
    pub humi:u8,
    pub temp:u8,
    pub light:f32,
    pub soil:u16,
}

///创建static 结构体用来给存储屏幕绘制的数据。
/// 不采用信道是因为传感器收集数据比较慢，生产者慢，而消费者消费很快。会阻塞tx的上传任务，导致用户误判
///由于这个static只在屏幕绘制使用所以不需要加互斥锁。
pub static mut SENSOR_COLLECT_DATA:DataCollect=DataCollect{
    humi:0,
    temp:0,
    light:0.0,
    soil:0,
};
#[task]
pub async fn control(
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,4>,
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
        let (dht11_humi, dht11_temp)=dht11_data;

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
        //保存信道的值在static中来给屏幕绘制
        unsafe {
            SENSOR_COLLECT_DATA.humi= dht11_humi;
            SENSOR_COLLECT_DATA.temp= dht11_temp;
            SENSOR_COLLECT_DATA.light=bh1750_data;
            SENSOR_COLLECT_DATA.soil=soil_data;
        }

        let water=unsafe{ACTUATOR_STATUS.water};
        let fan=unsafe{ACTUATOR_STATUS.fan};
        let light=unsafe{ACTUATOR_STATUS.light};
        let buzzer=unsafe{ACTUATOR_STATUS.buzzer};
        
        let json_part: String<128> = format!(
            "{{\"type\":\"data\",\"temp\":{},\"humi\":{},\"soil\":{},\"lux\":{},\"water\":{},\"light\":{},\"fan\":{},\"buzzer\":{}}}",
            dht11_temp as f32,
            dht11_humi as f32,
            soil_data,
            bh1750_data,
            water,
            light,
            fan,
            buzzer
        ).unwrap();
        
        let frame: String<128> = format!("{}\r\n", json_part).unwrap();
        tx_sender.send(frame).await;
    }
}

#[task]
pub async fn sub_control(
    rx_receiver:Receiver<'static,CriticalSectionRawMutex,String<128>,2>,
    tx_sender:Sender<'static,CriticalSectionRawMutex,String<128>,4>,
    mut actuator:Actuator<'static>,
){
    //状态机：记录每个设备预定关闭时间的时刻点
    //None 表示不需要自动关闭
    //Some(t) 表示需要在t时刻关闭
    let mut deadline_water:Option<Instant>=None;
    let mut deadline_fan:Option<Instant>=None;
    let mut deadline_light:Option<Instant>=None;
    let mut deadline_buzzer:Option<Instant>=None;
    loop {
        //-------------第一步计算还要睡多久----------------
        let next_wake_time=[deadline_water,deadline_fan,deadline_light,deadline_buzzer]
            .iter()//遍历四个状态
            .filter_map(|&d|d) //过滤None
            .min(); //找到所有some中的最小值
        //创建“等待定时器”的任务future
        let time_future=async {
            match next_wake_time {
                Some(time) =>{
                    //存在定时任务
                    Timer::at(time).await;
                },
                None=>{
                    //没有定时就全部挂起.pending会被挂起，直到被select取消
                    core::future::pending::<()>().await;
                }
            }
        };
        //--------------第二步开始竞赛select------------------
        //receiver和timer_future竞赛
        match select(rx_receiver.receive(),time_future).await {
            //结果A信道先发送命令
            Either::First(rx_cmd_frame)=>{
                let temp=rx_cmd_frame.as_str();
                if let Ok((cmd,_))=serde_json_core::from_str::<Cmd>(temp){
                    let ack=cmd.analysis();
                    let (act,switch)=ack.analysis();
                    match switch {
                        Switch::On=>{
                            actuator.set_on(act).await;
                            match act {
                                Act::Water=>deadline_water=None,
                                Act::Fan=>deadline_fan=None,
                                Act::Light=>deadline_light=None,
                                Act::Buzzer=>deadline_buzzer=None,
                            }
                        },
                        Switch::Off=>{
                            actuator.set_off(act).await;
                            match act {
                                Act::Water=>deadline_water=None,
                                Act::Fan=>deadline_fan=None,
                                Act::Light=>deadline_light=None,
                                Act::Buzzer=>deadline_buzzer=None,
                            }
                        },
                        Switch::Pulse(secs)=>{
                            actuator.set_on(act).await;
                            let close_at=Instant::now()+Duration::from_secs(secs);
                            match act {
                                Act::Water=>deadline_water=Some(close_at),
                                Act::Fan=>deadline_fan=Some(close_at),
                                Act::Light=>deadline_light=Some(close_at),
                                Act::Buzzer=>deadline_buzzer=Some(close_at),
                            }
                        }
                    }
                    //发送ack回复
                    let frame=ack_frame_wrap(&ack);
                    tx_sender.send(frame).await;
                }
            },
            //结果B，时间先到了，证明期间没有新的命令发送过来
            Either::Second(_)=>{
                //记录当前时间
                let now=Instant::now();

                if let Some(d)=deadline_water{
                    if now>d{
                        actuator.set_off(Act::Water).await;
                        deadline_water=None;
                    }
                }
                if let Some(d)=deadline_fan{
                    if now>d{
                        actuator.set_off(Act::Fan).await;
                        deadline_fan=None;
                    }
                }
                if let Some(d)=deadline_light{
                    if now>d{
                        actuator.set_off(Act::Light).await;
                        deadline_light=None;
                    }
                }
                if let Some(d)=deadline_buzzer{
                    if now>d{
                        actuator.set_off(Act::Buzzer).await;
                        deadline_buzzer=None;
                    }
                }
            }
        }
    }
}

#[derive(Deserialize, Debug, defmt::Format)]
struct Cmd {
    r#type: String<16>,
    target: String<16>,
    action: String<16>,
    time: i32,
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
            "pulse" => Ack::Pulse(act,time as u64),
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

