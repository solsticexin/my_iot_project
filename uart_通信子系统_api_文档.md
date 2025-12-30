# UART 通信子系统 API 文档

本文档仅描述嵌入式端 Rust 代码中的 API 结构与模块职责。完整的通信协议细节（帧格式、命令字等）请参考 `通信协议_v1.0.md`。

## 1. 模块概览

UART 子系统负责处理与上位机（如 ESP-01S 或 PC）的串行通信。采用异步任务模型，基于 `embassy-stm32` 和 `embassy-sync` 实现。

**核心文件**:
*   `src/protocol.rs`: 定义消息类型、数据结构、TLV 格式。
*   `src/uart.rs`: 实现 UART 驱动任务 (RX/TX frame 编解码)。
*   `src/command.rs`: 实现命令分发与执行器控制逻辑。

## 2. 核心数据结构 (`src/protocol.rs`)

### 2.1 消息类型 (`MessageType`)
| 枚举值 | ID | 说明 |
| :--- | :--- | :--- |
| `SensorReport` | `0x01` | 传感器周期上报 |
| `ActuatorStatus`| `0x02` | 执行器状态反馈 |
| `Command` | `0x10` | 控制命令（下行） |
| `CommandAck` | `0x11` | 命令收到确认（ACK） |
| `Heartbeat` | `0x20` | 心跳包 |

### 2.2 标签定义 (`Tag`)
**SensorTag**:
*   `SoilMoisture (0x01)`: u16 (ADC Value)
*   `Temperature (0x02)`: i16 (0.01°C)
*   `Humidity (0x03)`: u16 (0.01%)
*   `LightIntensity (0x04)`: u16 (Lux)

**ActuatorTag**:
*   `Fan (0x10)`: 风扇
*   `Pump (0x11)`: 水泵
*   `Light (0x12)`: 补光灯
*   `Buzzer (0x13)`: 蜂鸣器

## 3. 任务接口 (`src/uart.rs`)

### 3.1 `uart_rx_task`
*   **功能**: 读取 UART RX DMA 缓冲区，自动断帧并解析。
*   **逻辑**: 识别 SOF (`0xAA`) -> 解析 LEN -> 校验 CRC -> 提取 Payload。
*   **输出**: 若收到 `Command` 帧，解析为 `ControlCommand` 并发送至 `COMMAND_CHANNEL`。

### 3.2 `uart_tx_task`
*   **功能**: 接收发送请求，编码为二进制帧并写入 UART TX DMA。
*   **输入**: 监听 `UART_TX_CHANNEL`。
*   **支持消息**: `TxMessage::Sensor`, `TxMessage::Actuator`, `TxMessage::Ack`.

## 4. 命令系统 (`src/command.rs`)

### 4.1 `endpoint`
*   **通道**: `COMMAND_CHANNEL` (接收上位机指令), `uart_tx_channel` (发送 ACK/Feedback).

### 4.2 控制逻辑
*   **状态控制**: `Command Payload` 包含 `State` (ON/OFF) 和 `Duration`。
*   **脉冲模式**: 若 `Duration > 0`，则开启指定毫秒后自动关闭，并再次上报 OFF 状态。
