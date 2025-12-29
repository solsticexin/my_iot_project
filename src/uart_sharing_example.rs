#![no_std]

use embassy_executor::{task, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_stm32::usart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_time::{Duration, Timer};
use defmt::info;

// 定义共享的TX端类型
pub type SharedTx<'d> = Mutex<CriticalSectionRawMutex, embassy_stm32::usart::Tx<'d>>;

// 全局共享TX端
pub static SHARED_TX: Mutex<CriticalSectionRawMutex, Option<embassy_stm32::usart::Tx<'static>>> = Mutex::new(None);

// 任务1：使用共享TX端发送数据
#[task]
pub async fn task_tx1() {
    loop {
        // 获取共享TX端
        let mut tx = SHARED_TX.lock().await;
        if let Some(tx) = &mut *tx {
            // 发送数据
            let data = b"Task1: Hello from TX1!\r\n";
            if tx.write(data).await.is_ok() {
                info!("Task1: Data sent successfully");
            }
        }
        // 释放TX端
        drop(tx);
        
        Timer::after(Duration::from_secs(2)).await;
    }
}

// 任务2：使用共享TX端发送数据
#[task]
pub async fn task_tx2() {
    loop {
        // 获取共享TX端
        let mut tx = SHARED_TX.lock().await;
        if let Some(tx) = &mut *tx {
            // 发送数据
            let data = b"Task2: Hello from TX2!\r\n";
            if tx.write(data).await.is_ok() {
                info!("Task2: Data sent successfully");
            }
        }
        // 释放TX端
        drop(tx);
        
        Timer::after(Duration::from_secs(2)).await;
    }
}

// 任务3：使用RX端接收数据
#[task]
pub async fn task_rx(mut rx: embassy_stm32::usart::Rx<'static>) {
    let mut buffer = [0u8; 128];
    
    loop {
        info!("TaskRX: Waiting for data...");
        match rx.read(&mut buffer).await {
            Ok(n) => {
                info!("TaskRX: Received {} bytes: {:?}", n, &buffer[..n]);
            }
            Err(e) => {
                info!("TaskRX: Error reading: {}", e);
            }
        }
    }
}

// 初始化串口并设置共享
pub async fn init_uart_and_share(
    uart: embassy_stm32::usart::Uart<'static>,
    spawner: Spawner
) {
    // 将UART分离为RX和TX
    let (rx, tx) = uart.split();
    
    // 设置共享TX端
    *SHARED_TX.lock().await = Some(tx);
    
    // 启动任务
    if let Err(e) = spawner.spawn(task_tx1()) {
        info!("Failed to spawn task_tx1: {}", e);
    }
    
    if let Err(e) = spawner.spawn(task_tx2()) {
        info!("Failed to spawn task_tx2: {}", e);
    }
    
    if let Err(e) = spawner.spawn(task_rx(rx)) {
        info!("Failed to spawn task_rx: {}", e);
    }
    
    info!("UART sharing initialized successfully");
}

// 如果需要在现有代码中使用，可以在主程序外调用这个函数
// 示例：如何从主程序中调用（不需要修改主程序）
/*
在main.rs中创建UART后，可以将其传递给一个初始化任务：

match spawner.spawn(init_uart_task(usart, spawner.clone())) {
    Ok(_) => (),
    Err(e) => {
        error!("Failed to spawn init_uart_task: {}", e);
    }
}

#[embassy_executor::task]
async fn init_uart_task(uart: Uart<'static>, spawner: Spawner) {
    uart_sharing_example::init_uart_and_share(uart, spawner).await;
}
*/
