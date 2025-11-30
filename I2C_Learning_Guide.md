# Rust 嵌入式 I2C 通信协议指南 (基于 Embassy & STM32F1)

## 1. 协议核心理论

I2C (Inter-Integrated Circuit) 是一种同步、半双工、多主从架构的串行通信协议。

### 1.1 物理层连接
* **SCL (Serial Clock Line):** 时钟线，由主机控制。
* **SDA (Serial Data Line):** 数据线，双向传输。
* **关键特性 (Open-Drain):** * I2C 引脚为**开漏输出**模式。
    * **必须**在 SCL 和 SDA 线上接**上拉电阻** (通常 4.7kΩ) 到 VCC。
    * 默认空闲状态为高电平。

### 1.2 通信时序
一次标准的通信流程如下：
1.  **START (S):** SCL 高电平时，SDA 下拉。
2.  **ADDRESS:** 主机发送 7-bit 设备地址 + 1-bit 读写位 (0=Write, 1=Read)。
3.  **ACK/NACK:** 接收方在第 9 个时钟周期拉低 SDA 表示应答 (ACK)。
4.  **DATA:** 传输 8-bit 数据，每字节后跟随一个 ACK。
5.  **STOP (P):** SCL 高电平时，SDA 上拉，结束通信。

---

## 2. 项目配置 (Cargo.toml)

针对 STM32F103C8T6 和 Embassy 框架的基础依赖配置。

```toml
[dependencies]
# Embassy 核心与 STM32 支持
embassy-stm32 = { version = "0.1", features = ["defmt", "stm32f103c8", "unstable-pac", "memory-x", "time-driver-any"] }
embassy-executor = { version = "0.6", features = ["task-arena-size-32768", "arch-cortex-m", "executor-thread", "defmt"] }
embassy-time = { version = "0.3", features = ["defmt", "defmt-timestamp-uptime"] }

# ARM Cortex-M 运行时基础
cortex-m = { version = "0.7", features = ["critical-section-single-core"] }
cortex-m-rt = "0.7"

# 日志与调试
defmt = "0.3"
defmt-rtt = "0.4"
panic-probe = { version = "0.3", features = ["print-defmt"] }
````

-----

## 3\. 实战代码实现 (src/main.rs)

此代码演示了如何在 STM32F103 上配置 I2C1 (PB6/PB7) 并读取从机寄存器。

```rust
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::i2c::{I2c, Config, Hertz};
use embassy_stm32::{bind_interrupts, peripherals, i2c};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// 1. 绑定中断处理函数 (必须步骤)
bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

// 配置常量
const SLAVE_ADDRESS: u8 = 0x68; // 示例地址 (如 MPU6050)
const TARGET_REG: u8 = 0x75;    // 示例寄存器 (如 WHO_AM_I)

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 系统初始化
    let p = embassy_stm32::init(Default::default());
    defmt::info!("系统初始化完成");

    // 2. I2C 配置
    // STM32F1 I2C1 对应 DMA1 通道 6(Tx) 和 7(Rx)
    let mut i2c = I2c::new(
        p.I2C1, 
        p.PB6, // SCL
        p.PB7, // SDA
        Irqs, 
        p.DMA1_CH6, 
        p.DMA1_CH7, 
        Hertz(100_000), // 100kHz 标准模式
        Config::default()
    );

    loop {
        let write_buf = [TARGET_REG];
        let mut read_buf = [0u8; 1];

        // 3. 执行 Write-Read 事务
        // 自动处理: Start -> Addr(W) -> Reg -> RepeatedStart -> Addr(R) -> Data -> Stop
        match i2c.write_read(SLAVE_ADDRESS, &write_buf, &mut read_buf).await {
            Ok(()) => {
                defmt::info!("读取成功，数据: 0x{:02x}", read_buf[0]);
            },
            Err(e) => {
                defmt::error!("I2C 通信失败: {:?}", e);
            }
        }

        Timer::after_millis(1000).await;
    }
}
```

-----

## 4\. 常见问题与调试 (Troubleshooting)

### 4.1 BusError / ArbitrationLost

  * **现象:** 程序报错总线错误，或者初始化后立即失败。
  * **原因:** **缺少上拉电阻**。I2C 总线处于浮空状态。
  * **解决:** 检查硬件，确保 SCL 和 SDA 都有 4.7kΩ 电阻上拉至 3.3V。

### 4.2 地址错误 (NACK)

  * **现象:** 一直收到 `NACK` 错误。
  * **原因:** 使用了 8-bit 地址而非 7-bit 地址。
  * **解决:** 如果数据手册给出 Read/Write 地址 (如 0xD0/0xD1)，请右移一位使用 (0x68)。

### 4.3 编译错误 (Feature Missing)

  * **现象:** `no such field PB6` 或 `DMA1_CH6` 未找到。
  * **原因:** `Cargo.toml` 中未指定具体的芯片型号 feature。
  * **解决:** 确保开启 `stm32f103c8` feature。

-----

## 5\. 进阶练习任务

**任务名称：I2C 地址扫描器**

编写一个程序，遍历 `0x01` 到 `0x7F` 的所有可能的 I2C 地址。

1.  对每个地址执行 `i2c.write(addr, &[])`。
2.  如果返回 `Ok`，打印 "Found device at 0x..."。
3.  如果返回 `Err`，忽略并继续。

<!-- end list -->

```

---
