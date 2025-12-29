#![no_std]

use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};

// 定义消息类型
pub struct Task1Data(u32);
pub struct Task2Data(u32);

// 创建通道用于任务间通信
pub static TASK1_CHANNEL: Channel<CriticalSectionRawMutex, Task1Data, 1> = Channel::new();
pub static TASK2_CHANNEL: Channel<CriticalSectionRawMutex, Task2Data, 1> = Channel::new();

// 任务1：生成数据并发送
#[task(priority = 1)] // 设置任务优先级为1
pub async fn task1() {
    loop {
        let data = Task1Data(42);
        info!("Task1 sending data: {}", data.0);
        TASK1_CHANNEL.sender().send(data).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}

// 任务2：生成数据并发送
#[task(priority = 1)] // 设置任务优先级为1
pub async fn task2() {
    loop {
        let data = Task2Data(100);
        info!("Task2 sending data: {}", data.0);
        TASK2_CHANNEL.sender().send(data).await;
        Timer::after(Duration::from_secs(1)).await;
    }
}

// 任务3：接收task1和task2的数据并处理
#[task(priority = 2)] // 设置更高的优先级为2
pub async fn task3() {
    loop {
        info!("Task3 waiting for data...");

        // 同时等待两个通道的数据
        let (task1_data, task2_data) = embassy_futures::join::join(
            TASK1_CHANNEL.receiver().recv(),
            TASK2_CHANNEL.receiver().recv(),
        )
        .await;

        info!(
            "Task3 received data: task1={}, task2={}",
            task1_data.0, task2_data.0
        );

        // 处理数据
        let result = task1_data.0 + task2_data.0;
        info!("Task3 processing result: {}", result);

        Timer::after(Duration::from_secs(1)).await;
    }
}

// 主函数：启动所有任务
#[embassy_executor::main]
async fn main_example(spawner: Spawner) {
    // 启动任务1和任务2
    if let Err(e) = spawner.spawn(task1()) {
        info!("Failed to spawn task1: {}", e);
    }

    if let Err(e) = spawner.spawn(task2()) {
        info!("Failed to spawn task2: {}", e);
    }

    // 启动任务3
    if let Err(e) = spawner.spawn(task3()) {
        info!("Failed to spawn task3: {}", e);
    }
}
