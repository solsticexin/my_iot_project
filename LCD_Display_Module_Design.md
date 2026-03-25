<file_path>
my_iot_project\LCD_Display_Module_Design.md
</file_path>

<edit_description>
创建LCD显示模块软件设计说明文件
</edit_description>

# LCD显示模块软件设计说明

## 1. 概述

LCD显示模块是`my_iot_project`项目中的核心显示组件，负责在ST7735S TFT LCD屏幕上实时显示传感器数据和系统状态。该模块采用模块化设计，支持中文和ASCII字符显示，具有良好的可扩展性和维护性。

### 1.1 功能特性

- **实时数据显示**：周期性更新温湿度、光照、土壤湿度等传感器数据
- **多语言支持**：内置16x16点阵中文字库，支持中文标签显示
- **字符显示**：支持ASCII数字字符（0-9和小数点）显示
- **异步操作**：基于Embassy异步框架，实现非阻塞显示更新
- **硬件抽象**：封装ST7735S驱动，提供高层显示接口

### 1.2 硬件规格

- **显示屏型号**：ST7735S TFT LCD
- **分辨率**：128x160像素
- **接口**：SPI通信协议
- **色彩**：RGB565格式（16位色深）
- **背光控制**：可选GPIO控制

## 2. 软件架构设计

### 2.1 模块结构

LCD显示模块由以下核心文件组成：

```
src/
├── display.rs          # 显示任务和高层接口
├── st7735.rs           # ST7735S硬件驱动
└── font.rs             # 字符点阵数据
```

### 2.2 设计模式

- **异步任务模式**：使用`#[embassy_executor::task]`实现独立的显示任务
- **状态机模式**：显示任务周期性更新，避免阻塞其他系统任务
- **门面模式**：`display.rs`提供统一的高层接口，屏蔽底层硬件细节
- **静态数据模式**：字符点阵数据使用静态常量，提高访问效率

### 2.3 任务调度

显示模块作为一个独立的异步任务运行：

```rust
#[embassy_executor::task]
pub async fn display_task(mut display: ST7735<'static>) {
    // 初始化显示屏
    // 绘制静态标签
    // 循环更新数据
}
```

## 3. 核心组件详解

### 3.1 ST7735驱动 (`st7735.rs`)

#### 3.1.1 结构体定义

```rust
pub struct ST7735<'d> {
    spi: Spi<'d, Async>,    // SPI接口
    res: Output<'d>,        // 复位引脚
    dc: Output<'d>,         // 数据/命令选择引脚
    cs: Output<'d>,         // 片选引脚
    bl: Option<Output<'d>>, // 背光控制引脚（可选）
}
```

#### 3.1.2 主要方法

- `init()`: 显示屏初始化序列
- `write_command(cmd)`: 发送命令字节
- `write_data(data)`: 发送数据字节
- `set_window(x0, y0, x1, y1)`: 设置显示窗口
- `clear_screen(color)`: 清屏
- `draw_chinese_char(x, y, bitmap, fg, bg)`: 绘制中文字符
- `draw_ascii_char(x, y, bitmap, fg, bg)`: 绘制ASCII字符

#### 3.1.3 初始化流程

1. 硬件复位（RES引脚）
2. 发送软件复位命令
3. 配置显示参数（帧率、像素格式等）
4. 设置内存访问控制
5. 开启显示

### 3.2 字符库 (`font.rs`)

#### 3.2.1 数据结构

- **中文字符**：16x16点阵，占据48像素宽度（6字节/行）
- **ASCII数字**：16x16点阵，占据16像素宽度（2字节/行）
- **存储格式**：按行存储，每行包含该行所有列的字节数据

#### 3.2.2 支持的字符

- **中文标签**：温度、湿度、土壤、光照
- **数字字符**：0-9和小数点

#### 3.2.3 查找函数

```rust
pub fn get_digit_bitmap(digit: char) -> Option<&'static [[u8; 2]; 16]>
```

### 3.3 显示任务 (`display.rs`)

#### 3.3.1 任务职责

1. 初始化显示屏硬件
2. 绘制静态标签（温度、湿度等）
3. 周期性读取传感器数据
4. 更新数值显示
5. 管理显示布局和色彩

#### 3.3.2 显示布局

```
+----------------+
| 温度: XX.X     |
| 湿度: XX.X     |
| 土壤: XXX      |
| 光照: XXX.X    |
+----------------+
```

#### 3.3.3 数据更新策略

- **周期更新**：每3秒刷新一次显示
- **数据源**：从全局静态变量`SENSOR_COLLECT_DATA`读取
- **格式化**：使用`heapless::String`进行数值格式化

## 4. 接口设计

### 4.1 公共接口

#### ST7735结构体方法

```rust
impl<'d> ST7735<'d> {
    pub fn new(...) -> Self
    pub async fn init(&mut self) -> Result<(), Error>
    pub async fn clear_screen(&mut self, color: u16) -> Result<(), Error>
    pub async fn draw_chinese_char(&mut self, ...) -> Result<(), Error>
    pub async fn draw_ascii_char(&mut self, ...) -> Result<(), Error>
}
```

#### 字符库接口

```rust
pub const CHAR_TEMPERATURE: [[u8; 6]; 16]
pub const CHAR_HUMIDITY: [[u8; 6]; 16]
// ... 其他字符常量

pub fn get_digit_bitmap(digit: char) -> Option<&'static [[u8; 2]; 16]>
```

### 4.2 内部接口

#### 显示任务函数

```rust
pub async fn display_task(mut display: ST7735<'static>)
```

#### 辅助函数

```rust
async fn draw_temperature(display: &mut ST7735<'_>, x: u16, y: u16, fg: u16, bg: u16)
async fn draw_humidity(display: &mut ST7735<'_>, x: u16, y: u16, fg: u16, bg: u16)
// ... 其他绘制函数

async fn draw_digits(display: &mut ST7735<'_>, x: u16, y: u16, text: &str, spacing: u16, fg: u16, bg: u16)
```

## 5. 数据流程

### 5.1 数据采集流程

1. 各传感器任务采集数据
2. 数据通过Channel发送到控制中心
3. 控制中心更新全局静态变量`SENSOR_COLLECT_DATA`
4. 显示任务读取静态变量进行显示

### 5.2 显示更新流程

```
传感器数据采集 → 控制中心处理 → 全局静态变量更新 → 显示任务读取 → 屏幕刷新
```

### 5.3 字符渲染流程

1. 获取字符点阵数据
2. 设置显示窗口
3. 逐像素写入颜色数据
4. SPI传输到显示屏

## 6. 实现细节

### 6.1 SPI通信

- **频率**：15MHz
- **模式**：SPI Mode 0
- **数据格式**：8位传输
- **DMA支持**：使用DMA提高传输效率

### 6.2 色彩系统

- **格式**：RGB565 (16位)
- **常量定义**：
  - `COLOR_BLACK: 0x0000`
  - `COLOR_WHITE: 0xFFFF`
  - `COLOR_RED: 0xF800`
  - `COLOR_YELLOW: 0xFFE0`

### 6.3 内存管理

- **静态常量**：字符点阵数据存储在只读内存
- **堆分配**：使用`heapless::String`避免动态内存分配
- **栈优化**：及时释放临时变量，避免栈溢出

### 6.4 错误处理

- **异步错误**：使用`Result<(), embassy_stm32::spi::Error>`
- **日志记录**：通过`defmt`记录初始化和通信错误
- **容错设计**：显示任务失败不影响其他系统功能

## 7. 配置和初始化

### 7.1 硬件配置

```rust
// SPI配置
let mut spi_config = spi::Config::default();
spi_config.frequency = mhz(15);

// 引脚配置
let cs = Output::new(p.PA3, Level::Low, Speed::VeryHigh);
let dc = Output::new(p.PA4, Level::High, Speed::VeryHigh);
let rst = Output::new(p.PA2, Level::Low, Speed::VeryHigh);

// 创建显示实例
let display = ST7735::new(spi_async, rst, dc, cs, None);
```

### 7.2 任务启动

```rust
spawner.spawn(display::display_task(display));
```

## 8. 性能优化

### 8.1 显示效率

- **窗口设置**：只刷新需要更新的区域
- **增量更新**：只更新变化的数值部分
- **DMA传输**：使用硬件DMA提高数据传输速度

### 8.2 内存优化

- **常量数据**：字符点阵存储为静态常量
- **无堆分配**：使用固定大小的堆分配字符串
- **局部变量**：及时释放临时变量

### 8.3 功耗控制

- **背光管理**：支持背光开启/关闭
- **刷新频率**：3秒更新周期平衡显示效果和功耗

## 9. 扩展性设计

### 9.1 新字符支持

1. 在`font.rs`中添加新的字符常量
2. 定义对应的点阵数据数组
3. 在显示任务中添加绘制调用

### 9.2 显示布局调整

- 修改坐标参数调整标签位置
- 调整颜色常量改变显示风格
- 添加新的绘制函数支持更多元素

### 9.3 多屏支持

- 抽象显示接口
- 实现不同的显示驱动
- 通过配置选择不同的显示硬件

## 10. 测试和调试

### 10.1 单元测试

- **硬件测试**：验证SPI通信和引脚连接
- **字符测试**：验证点阵数据显示正确性
- **性能测试**：测量显示刷新时间

### 10.2 调试功能

- **日志输出**：使用`defmt`记录显示操作
- **状态监控**：监控显示任务运行状态
- **错误诊断**：定位显示异常原因

## 11. 总结

LCD显示模块采用了分层架构设计，从硬件驱动层到高层显示接口，实现了模块化和可维护性。异步设计确保了显示操作不阻塞系统其他任务，静态数据存储提高了运行效率。该模块为整个IoT系统提供了直观的用户界面，支持实时数据显示和系统状态监控。

---

**文档版本**：1.0  
**最后更新**：2024年  
**作者**：项目开发团队  
**版权**：保留所有权利