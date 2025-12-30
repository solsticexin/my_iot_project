# 嵌入式自动化节点 API 文档（Embassy + UART + TLV）

本 API 文档总结了系统的**整体架构、任务职责、串口协议、事件模型与数据流**。目标是构建一个**可靠、可扩展、可调试**的嵌入式自动化控制节点。

---

## 1. 系统概览

系统由 MCU（Embassy 异步运行时）与 ESP-01S 通过 UART 通信组成。

功能包括：
- 采集三类传感器数据
  - 土壤湿度
  - 环境温度 / 湿度
  - 光照强度
- 控制四类执行器
  - 风扇（继电器，高电平触发）
  - 水泵（继电器，高电平触发）
  - 补光灯（继电器，高电平触发）
  - 蜂鸣器（低电平触发）
- 支持执行器的**即时控制**与**定时 Pulse 控制**
- 所有状态与数据通过统一 UART 帧协议上传

---

## 2. 设计核心思想

### 2.1 逻辑分层

- **系统内部（职责明确）**
  - 传感器：观察世界（时间驱动）
  - 执行器：改变世界（事件驱动）
- **通信层（统一出口）**
  - UART 只负责“搬运消息”，不关心业务含义

### 2.2 事件模型

- 传感器数据：周期性上报（可覆盖、可丢帧）
- 执行器状态：事件触发上报（状态变化即上报）
- 串口不是调度者，只是通道

---

## 3. 任务（Task）划分

### 3.1 uart_rx_task

**职责**：
- 从 UART 接收字节流
- 进行帧同步（SOF）
- 校验 LEN / CRC
- 解析 TYPE + PAYLOAD
- 将解析后的命令发送至 command_task

**特性**：
- RX only
- 不关心业务含义

---

### 3.2 uart_tx_task

**职责**：
- 从 tx_channel 接收待发送消息
- 按帧格式编码
- 顺序发送到 UART

**特性**：
- TX only
- 严格顺序保证
- 系统唯一 UART 发送出口

---

### 3.3 command_task

**职责**：
- 接收来自 uart_rx_task 的 Command
- 校验参数合法性
- 转发给 actuator_task 执行
- 生成 CommandAck（成功 / 失败）

**特性**：
- 不直接操作 GPIO
- 不处理定时

---

### 3.4 sensor_task_x（多个）

**职责**：
- 周期性采集对应传感器
- 生成 SensorReport
- 发送到 tx_channel

**特性**：
- 时间驱动
- 周期固定
- 数据可合并、可限流

---

### 3.5 actuator_task（逻辑存在）

**职责**：
- 接收执行器控制命令
- 操作 GPIO（内部处理高/低电平差异）
- 管理 Pulse 定时
- 在状态变化时生成 ActuatorStatus 上报

**触发上报的时机**：
- 命令执行完成
- Pulse 结束
- 系统启动初始化完成

---

## 4. 串口帧格式（TLV 风格）

```
+--------+--------+--------+----------+--------+
|  SOF   |  LEN   |  TYPE  | PAYLOAD  |  CRC   |
+--------+--------+--------+----------+--------+
1 byte   1 byte   1 byte   N bytes    1 byte
```

- **SOF**：帧起始标志（固定值）
- **LEN**：TYPE + PAYLOAD 长度
- **TYPE**：帧语义类型
- **PAYLOAD**：TLV 编码数据
- **CRC**：简单校验（CRC8 或 XOR）

---

## 5. TYPE 定义（语义多路复用）

```
0x01  SensorReport
0x02  ActuatorStatus
0x10  Command
0x11  CommandAck
0x20  Heartbeat / SystemStatus（可选）
```

TYPE 表示“这帧在做什么”，而不是“来自哪个模块”。

---

## 6. Payload：TLV 结构

```
+------+--------+---------+
| TAG  | LENGTH |  VALUE  |
+------+--------+---------+
```

### 6.1 传感器 TAG

```
0x01  SoilMoisture   (u16)
0x02  Temperature    (i16, 0.01°C)
0x03  Humidity       (u16, 0.01%)
0x04  LightIntensity (u16)
```

SensorReport 中可只包含“本次存在的数据”。

---

### 6.2 执行器 TAG

```
0x10  Fan
0x11  Pump
0x12  Light
0x13  Buzzer
```

VALUE：
- `0x00` = OFF
- `0x01` = ON

---

### 6.3 Command Payload

```
[TAG][1][state]
[TAG][2][duration_ms]
```

- state = 0x00 / 0x01
- duration_ms 用于 Pulse 控制

---

## 7. 上报策略总结

### 7.1 传感器

- 周期性上报（Timer 驱动）
- 不关心命令
- 可合并多个传感器到一个帧

### 7.2 执行器

- 仅在状态变化时上报
- 不周期发送
- 事件驱动（命令 / Pulse / 初始化）

---

## 8. 数据流全景图（逻辑）

```
sensor_task_x ─┐
               ├─> tx_channel ─> uart_tx_task ─> UART
actuator_task ─┘

UART ─> uart_rx_task ─> command_task ─> actuator_task
```

---

## 9. 设计原则总结

- 串口是通道，不是调度器
- TYPE 是语义，不是模块
- 传感器 = 时间驱动
- 执行器 = 事件驱动
- 谁最接近事实，谁负责上报

---

## 10. 工程目标

该 API 设计目标是：
- 清晰的因果关系
- 易于扩展新设备
- 丢帧可恢复
- 抓串口可读、可调试

这不是“数据传输协议”，而是一个**消息驱动的控制系统接口**。
