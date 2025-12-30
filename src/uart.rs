use crate::config::{SHARED_TX, UART_TX_CHANNEL};
use crate::protocol::{ActuatorTag, MessageType, SOF, SensorData, SensorTag, TxMessage};
use embassy_executor::task;
use embassy_stm32::{mode::Async, usart::UartRx};
use embassy_time::{Duration, with_timeout};

/// 最小帧长 SOF + LEN + TYPE + CRC (Payload为0时)
const MIN_FRAME_LEN: usize = 4;

/// 校验结果枚举
pub enum FrameError {
    HeaderError,  // 头不对
    Incomplete,   // 数据没收完
    CrcError,     // 校验失败
    Valid(usize), // 合法帧，返回帧的总长度
}

/// 核心校验函数
/// data: 收到的原始 buffer
pub fn check_frame(data: &[u8]) -> FrameError {
    let received_len = data.len();

    // 1. 基础长度检查
    if received_len < MIN_FRAME_LEN {
        return FrameError::Incomplete;
    }

    // 2. 检查 SOF (帧头)
    if data[0] != SOF {
        return FrameError::HeaderError;
    }

    // 3. 计算理论上的总长度
    // LEN 字段 = TYPE(1) + PAYLOAD(N)
    // 总帧长 = SOF(1) + LEN字段(1) + LEN的值 + CRC(1)
    let body_len = data[1] as usize;
    let expected_total_len = 1 + 1 + body_len + 1;

    // 4. 检查是否接收完整
    if received_len < expected_total_len {
        return FrameError::Incomplete;
    }

    // 5. 校验 CRC
    // 范围：从 SOF 到 PAYLOAD 结束
    let frame_content = &data[0..expected_total_len - 1];
    let received_crc = data[expected_total_len - 1];

    let calculated_crc = calculate_crc(frame_content);

    if calculated_crc != received_crc {
        return FrameError::CrcError;
    }

    FrameError::Valid(expected_total_len)
}

/// 简单的异或校验 (XOR Checksum)
fn calculate_crc(data: &[u8]) -> u8 {
    let mut crc = 0;
    for &byte in data {
        crc ^= byte;
    }
    crc
}

#[task]
pub async fn uart_tx_task() {
    let receiver = UART_TX_CHANNEL.receiver();
    loop {
        let msg = receiver.receive().await;
        // 最大帧长估计：SensorReport 有 4 个传感器数据，每个 3 byte (tag+len+val?) no, value is 8 bytes in TLVItem but defined strictly.
        // Let's simple buffer
        let mut buffer = [0u8; 64];
        let len = encode_msg(&msg, &mut buffer);

        if len > 0 {
            let mut tx_lock = SHARED_TX.lock().await;
            if let Some(tx) = tx_lock.as_mut() {
                // 发送
                if let Err(e) = tx.write(&buffer[..len]).await {
                    crate::fmt::warn!("UART TX Error: {}", e);
                }
            }
        }
    }
}

fn encode_msg(msg: &TxMessage, buffer: &mut [u8]) -> usize {
    // 构造 Payload
    // Frame: SOF, LEN, TYPE, Payload..., CRC
    // Index: 0    1    2     3...

    buffer[0] = SOF;
    // buffer[1] = LEN (filled later)
    // buffer[2] = TYPE (filled later)

    let mut payload_idx = 3;
    let msg_type;

    match msg {
        TxMessage::Sensor(data) => {
            msg_type = MessageType::SensorReport;
            match data {
                SensorData::SoilMoisture(val) => append_tlv_u16(
                    buffer,
                    &mut payload_idx,
                    SensorTag::SoilMoisture as u8,
                    *val,
                ),
                SensorData::Temperature(val) => {
                    append_tlv_i16(buffer, &mut payload_idx, SensorTag::Temperature as u8, *val)
                }
                SensorData::Humidity(val) => {
                    append_tlv_u16(buffer, &mut payload_idx, SensorTag::Humidity as u8, *val)
                }
                SensorData::LightIntensity(val) => append_tlv_u16(
                    buffer,
                    &mut payload_idx,
                    SensorTag::LightIntensity as u8,
                    *val,
                ),
            }
        }
        TxMessage::Actuator(status) => {
            msg_type = MessageType::ActuatorStatus;
            // Tag
            buffer[payload_idx] = status.actuator as u8;
            payload_idx += 1;
            // Len
            buffer[payload_idx] = 1;
            payload_idx += 1;
            // Value
            buffer[payload_idx] = if status.state { 1 } else { 0 };
            payload_idx += 1;
        }
        TxMessage::Ack(ack) => {
            msg_type = MessageType::CommandAck;
            // Tag
            buffer[payload_idx] = ack.actuator as u8;
            payload_idx += 1;
            // Len
            buffer[payload_idx] = 1;
            payload_idx += 1;
            // Value (0x01 Success, 0x00 Fail)
            buffer[payload_idx] = if ack.success { 1 } else { 0 };
            payload_idx += 1;
        }
        TxMessage::Heartbeat => {
            msg_type = MessageType::Heartbeat;
        }
    }

    buffer[2] = msg_type as u8;

    // Calculate LEN = TYPE(1) + PAYLOAD
    let payload_len = payload_idx - 3; // buffer[3] is start of payload
    let total_body_len = 1 + payload_len;
    buffer[1] = total_body_len as u8;

    // CRC
    let crc_idx = payload_idx;
    buffer[crc_idx] = calculate_crc(&buffer[0..crc_idx]);

    crc_idx + 1 // Total length
}

fn append_tlv_u16(buffer: &mut [u8], idx: &mut usize, tag: u8, val: u16) {
    buffer[*idx] = tag;
    *idx += 1;
    buffer[*idx] = 2; // Len
    *idx += 1;
    let bytes = val.to_be_bytes();
    buffer[*idx] = bytes[0];
    *idx += 1;
    buffer[*idx] = bytes[1];
    *idx += 1;
}

fn append_tlv_i16(buffer: &mut [u8], idx: &mut usize, tag: u8, val: i16) {
    buffer[*idx] = tag;
    *idx += 1;
    buffer[*idx] = 2; // Len
    *idx += 1;
    let bytes = val.to_be_bytes();
    buffer[*idx] = bytes[0];
    *idx += 1;
    buffer[*idx] = bytes[1];
    *idx += 1;
}

#[task]
pub async fn uart_rx_task(mut rx: UartRx<'static, Async>) {
    let mut buffer = [0u8; 128];
    // Start index of valid data
    let mut valid_start = 0;
    // End index of valid data
    let mut valid_end = 0;

    // Command Sender
    let cmd_sender = crate::config::COMMAND_CHANNEL.sender();

    loop {
        // Read into buffer after valid_end
        // We must ensure we have space
        if valid_end >= buffer.len() {
            // Buffer full, discard or shift?
            // Should have shifted already. If full here, means we have a huge packet or garbage.
            // Reset.
            valid_start = 0;
            valid_end = 0;
        }

        match with_timeout(
            Duration::from_secs(1),
            rx.read_until_idle(&mut buffer[valid_end..]),
        )
        .await
        {
            Ok(Ok(len)) => {
                if len == 0 {
                    continue;
                }
                valid_end += len;
            }
            Ok(Err(e)) => {
                crate::fmt::warn!("RX Error: {}", e);
                continue;
            }
            Err(_) => {
                // Timeout
            }
        }

        // Process buffer
        while valid_end - valid_start >= MIN_FRAME_LEN {
            let data_slice = &buffer[valid_start..valid_end];

            // Check potential frame
            match check_frame(data_slice) {
                FrameError::Valid(len) => {
                    // Valid frame found at valid_start with length len
                    let frame_len = len;
                    let frame = &buffer[valid_start..valid_start + frame_len];

                    // Parse Frame
                    // Frame: SOF(1) LEN(1) TYPE(1) Payload(N) CRC(1)
                    let msg_type_byte = frame[2];
                    let payload_len = frame[1] as usize - 1; // LEN = TYPE + Payload
                    let payload = &frame[3..3 + payload_len];

                    if msg_type_byte == MessageType::Command as u8 {
                        // Parse Command Payload
                        parse_and_send_commands(payload, &cmd_sender).await;
                    }

                    // Consume frame
                    valid_start += len;
                }
                FrameError::HeaderError => {
                    // Current byte is not SOF, skip 1
                    valid_start += 1;
                }
                FrameError::CrcError => {
                    // Header looked ok (SOF correct), but CRC failed.
                    // It might be a false SOF detection or corrupted frame.
                    // Skip 1 byte and try to resync.
                    valid_start += 1;
                }
                FrameError::Incomplete => {
                    // Need more data
                    break;
                }
            }
        }

        // Compact buffer
        if valid_start > 0 {
            if valid_start == valid_end {
                valid_start = 0;
                valid_end = 0;
            } else {
                // only move if necessary or if close to end
                if valid_end > 100 {
                    let len = valid_end - valid_start;
                    buffer.copy_within(valid_start..valid_end, 0);
                    valid_start = 0;
                    valid_end = len;
                }
            }
        }
    }
}

async fn parse_and_send_commands(
    payload: &[u8],
    sender: &embassy_sync::channel::Sender<
        '_,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        crate::protocol::ControlCommand,
        4,
    >,
) {
    let mut i = 0;
    while i < payload.len() {
        if i + 2 > payload.len() {
            break;
        } // Need at least Tag + Len
        let tag = payload[i];
        let len = payload[i + 1];
        let val_start = i + 2;
        let val_end = val_start + len as usize;

        if val_end > payload.len() {
            break;
        }

        let value_bytes = &payload[val_start..val_end];

        // Try to map Tag to Actuator
        let actuator = ActuatorTag::from(tag);
        // Note: ActuatorTag::from defaults to Fan if unknown.
        // Ideally we should check if tag is valid.
        // Assuming valid for now or filtered by ActuatorTag definition range if we improved From impl.

        // Heuristic:
        // Len == 1 => State (0/1)
        // Len == 2 => Duration (u16)

        let mut cmd = crate::protocol::ControlCommand {
            actuator,
            state: false,
            duration_ms: 0,
        };

        if len == 1 {
            cmd.state = value_bytes[0] != 0;
            // duration default 0
            sender.send(cmd).await;
        } else if len == 2 {
            let val = u16::from_be_bytes([value_bytes[0], value_bytes[1]]);
            cmd.duration_ms = val;
            cmd.state = true; // Duration implies ON?
            sender.send(cmd).await;
        }

        // Move to next TLV
        i = val_end;
    }
}
