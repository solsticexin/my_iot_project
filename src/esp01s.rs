use embassy_stm32::{mode, usart};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json_core::heapless::String;

///esp-01s错误类型
pub enum Esp01sError {
    Json(serde_json_core::ser::Error),
    Uart(usart::Error),
    FrameTypeError(crate::esp01s::FrameError),
}

type FrameError = FrameTypeError;

pub enum FrameTypeError {
    NotReport,
    NotCommand,
    NotReceipt,
}

///esp01s通信数据帧类型
pub enum FrameType {
    ///数据上报帧
    DataReport(DataReportFrame),
    ///命令执行帧
    CommandExecute(CommandExecuteFrame),
    ///执行回执帧
    ExecutionReceipt(ExecutionReceiptFrame),
}
impl FrameType {
    #[inline]
    pub fn analysis_report(self) -> Result<DataReportFrame, FrameError> {
        if let FrameType::DataReport(val) = self {
            Ok(val)
        } else {
            Err(FrameError::NotReport)
        }
    }
    #[inline]
    pub fn analysis_command(self) -> Result<CommandExecuteFrame, FrameError> {
        if let FrameType::CommandExecute(val) = self {
            Ok(val)
        } else {
            Err(FrameError::NotCommand)
        }
    }
    #[inline]
    pub fn analysis_receipt(self) -> Result<ExecutionReceiptFrame, FrameError> {
        if let FrameType::ExecutionReceipt(val) = self {
            Ok(val)
        } else {
            Err(FrameError::NotReceipt)
        }
    }
}

pub struct Esp01s<'d> {
    usart: usart::Uart<'d, mode::Async>,
}

impl<'d> Esp01s<'d> {
    pub fn new(usart: usart::Uart<'d, mode::Async>) -> Self {
        Self { usart }
    }
    pub async fn data_report(&mut self, data: FrameType) -> Result<(), Esp01sError> {
        let mut frame = data
            .analysis_report()
            .map_err(Esp01sError::FrameTypeError)?;
        // let mut frame = match data {
        //     DataReport(frame) => frame,
        //     _ => return Err(Esp01sError::FrameTypeError),
        // };
        let frame = frame.to_json().map_err(Esp01sError::Json)?;
        self.usart
            .write(frame.as_bytes())
            .await
            .map_err(Esp01sError::Uart)
    }
    pub async fn command_execute(&mut self, command: FrameType) -> Result<(), Esp01sError> {
        let mut frame = command
            .analysis_command()
            .map_err(Esp01sError::FrameTypeError)?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct DataReportFrame {
    temp: u8,     //温度
    humi: u8,     //湿度
    soil: u8,     //土壤湿度
    lux: u16,     //光照强度
    water: bool,  //水泵继电开关
    light: bool,  //补光灯继电器开关
    fan: bool,    //风扇继电器开关
    buzzer: bool, //蜂鸣器继电器开关
}
impl DataReportFrame {
    pub fn new(
        temp: u8,
        humi: u8,
        soil: u8,
        lux: u16,
        water: bool,
        light: bool,
        fan: bool,
        buzzer: bool,
    ) -> Self {
        Self {
            temp,
            humi,
            soil,
            lux,
            water,
            light,
            fan,
            buzzer,
        }
    }
    pub fn to_json(&mut self) -> serde_json_core::ser::Result<String<128>> {
        serde_json_core::to_string(&self)
    }
}
#[derive(Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CommandExecuteFrame {
    pub target: target,
    pub action: Action,
}

#[derive(Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum target {
    Water,
    Light,
    Fan,
    Buzzer,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Action {
    On,
    Off,
    Pulse(u16), // 脉冲时间(ms)
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::<32>::deserialize(deserializer)?;
        if s == "On" {
            Ok(Action::On)
        } else if s == "Off" {
            Ok(Action::Off)
        } else if s.starts_with("Pulse(") && s.ends_with(')') {
            let duration_str = &s[6..s.len() - 1];
            match duration_str.parse::<u16>() {
                Ok(duration) => Ok(Action::Pulse(duration)),
                Err(_) => Err(D::Error::custom("Invalid pulse duration")),
            }
        } else {
            Err(Error::custom("Invalid Action"))
        }
    }
}

pub struct ExecutionReceiptFrame {
    target: target,
    action: Action,
    result: bool,
    message: ExecutionReceiptMessage,
}
pub enum ExecutionReceiptMessage {
    Success,
    Failed,
}
