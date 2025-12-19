# My IoT Project (STM32F103)

本项目是一个基于 STM32F103 的 IoT 嵌入式系统项目，使用 Rust 语言和 Embassy 异步框架开发。

## 功能特性

*   **异步架构**: 基于 `embassy-stm32` 和 `embassy-executor`，完全发挥 Rust 异步特性。
*   **外设驱动**:
    *   **ST7735S Display**: 1.8寸 TFT 屏幕驱动 (SPI)，支持 `embedded-graphics` 绘图库。
    *   **DH11**: 温湿度传感器驱动 (GPIO)。
    *   **BH1750**: 光照传感器驱动 (I2C)。
*   **任务调度**: 使用 Embassy Executor 管理多个并发任务 (显示、传感器读取等)。

## 硬件连接

### ST7735S 屏幕 (SPI)
*   **SCL (SCK)** -> PA5
*   **SDA (MOSI)** -> PA7
*   **RES (Reset)** -> PA2
*   **DC (Data/Command)** -> PA4
*   **CS (Chip Select)** -> PA3

### BH1750 光照传感器 (I2C)
*   **SCL** -> PB6
*   **SDA** -> PB7

### DH11 温湿度传感器
*   **DATA** -> PB11

## 软件模块说明

*   `src/main.rs`: 程序入口，负责硬件初始化和任务生成 (Spawning tasks)。
    *   `test_st7735_task`: 演示屏幕绘图功能 (圆、清屏、方向设置)。
    *   `dh11_task`: 读取温湿度数据。
    *   `bh1750_read`: 读取光照强度。
*   `src/st7735.rs`: ST7735S 屏幕驱动核心实现。
    *   提供初始化、清屏、方向设置、偏移设置。
    *   实现 `draw_pixels` 接口适配 `embedded-graphics`。
*   `src/fmt.rs`: 格式化与日志输出工具 (Defmt/Panic handler)。

## 快速开始

### 依赖
确保已安装 Rust 工具链和 `thumbv7m-none-eabi` 目标：
```bash
rustup target add thumbv7m-none-eabi
cargo install probe-rs
```

### 编译与运行
使用 cargo 编译并烧录：
```bash
cargo run --release
```

## 注意事项
*   本项目使用 `defmt` 进行日志输出，需要配合 `probe-rs` 或类似工具查看日志。
*   ST7735 驱动针对 128x160 分辨率屏幕优化，如使用不同分辨率可能需要调整 `src/st7735.rs` 中的 `set_offset` 或 `Resolution` 设置。
