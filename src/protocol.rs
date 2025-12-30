#![allow(dead_code)]

/// 帧起始标志
pub const SOF: u8 = 0xAA;

/// 消息类型定義
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    SensorReport = 0x01,
    ActuatorStatus = 0x02,
    Command = 0x10,
    CommandAck = 0x11,
    Heartbeat = 0x20,
    Unknown = 0xFF,
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => MessageType::SensorReport,
            0x02 => MessageType::ActuatorStatus,
            0x10 => MessageType::Command,
            0x11 => MessageType::CommandAck,
            0x20 => MessageType::Heartbeat,
            _ => MessageType::Unknown,
        }
    }
}

/// 传感器 TAG 定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SensorTag {
    SoilMoisture = 0x01,   // u16
    Temperature = 0x02,    // i16, 0.01°C
    Humidity = 0x03,       // u16, 0.01%
    LightIntensity = 0x04, // u16
}

/// 执行器 TAG 定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActuatorTag {
    Fan = 0x10,
    Pump = 0x11,
    Light = 0x12,
    Buzzer = 0x13,
}

impl From<u8> for ActuatorTag {
    fn from(value: u8) -> Self {
        match value {
            0x10 => ActuatorTag::Fan,
            0x11 => ActuatorTag::Pump,
            0x12 => ActuatorTag::Light,
            0x13 => ActuatorTag::Buzzer,
            _ => ActuatorTag::Fan, // 默认或错误处理
        }
    }
}

/// 通用的 TLV 结构用于构建 Payload
#[derive(Debug)]
pub struct TlvItem {
    pub tag: u8,
    pub length: u8,
    pub value: [u8; 8], // 最大 8 字节，足够容纳目前所有数据类型
}

impl TlvItem {
    pub fn new_u16(tag: u8, val: u16) -> Self {
        let bytes = val.to_be_bytes(); // 网络字节序 (Big Endian) 推荐
        let mut value = [0u8; 8];
        value[0] = bytes[0];
        value[1] = bytes[1];
        Self {
            tag,
            length: 2,
            value,
        }
    }

    pub fn new_i16(tag: u8, val: i16) -> Self {
        let bytes = val.to_be_bytes();
        let mut value = [0u8; 8];
        value[0] = bytes[0];
        value[1] = bytes[1];
        Self {
            tag,
            length: 2,
            value,
        }
    }

    pub fn new_u8(tag: u8, val: u8) -> Self {
        let mut value = [0u8; 8];
        value[0] = val;
        Self {
            tag,
            length: 1,
            value,
        }
    }
}

/// 传感器上报数据结构 (内部使用)
#[derive(Debug, Clone, Copy)]
pub enum SensorData {
    SoilMoisture(u16),
    Temperature(i16),
    Humidity(u16),
    LightIntensity(u16),
}

/// 执行器控制命令
#[derive(Debug, Clone, Copy)]
pub struct ControlCommand {
    pub actuator: ActuatorTag,
    pub state: bool,      // true = ON, false = OFF
    pub duration_ms: u16, // 0 = 永久, >0 = Pulse
}

#[derive(Debug, Clone, Copy)]
pub struct ActuatorFeedback {
    pub actuator: ActuatorTag,
    pub state: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandAck {
    pub actuator: ActuatorTag,
    pub success: bool,
}

/// 发送到 UART TX 任务的统一消息枚举
#[derive(Debug, Clone, Copy)]
pub enum TxMessage {
    Sensor(SensorData),
    Actuator(ActuatorFeedback),
    Ack(CommandAck),
    Heartbeat,
}
