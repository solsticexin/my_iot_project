#![no_std]

use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use embassy_time::{Duration, Timer};

// 方法1：使用Channel实现任务顺序执行
pub static TASK4_COMPLETE: Channel<CriticalSectionRawMutex, (), 1> = Channel::new();

// 方法2：使用Signal实现任务顺序执行
pub static TASK4_DONE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// 假设这是现有的task4，我们不能修改它
async fn existing_task4() {
    info!("Task4: Start executing");
    // 模拟task4的工作
    Timer::after(Duration::from_secs(2)).await;
    info!("Task4: Finished executing");
}

// 假设这是现有的task5，我们不能修改它
async fn existing_task5() {
    info!("Task5: Start executing");
    // 模拟task5的工作
    Timer::after(Duration::from_secs(1)).await;
    info!("Task5: Finished executing");
}

// 方法1：使用包装任务确保顺序执行 - Channel方式
#[task]
pub async fn wrapped_task4_channel() {
    // 执行原始的task4
    existing_task4().await;
    // 发送完成信号
    TASK4_COMPLETE.sender().send(()).await;
}

#[task]
pub async fn wrapped_task5_channel() {
    // 等待task4完成信号
    TASK4_COMPLETE.receiver().recv().await;
    // 执行原始的task5
    existing_task5().await;
}

// 方法2：使用包装任务确保顺序执行 - Signal方式
#[task]
pub async fn wrapped_task4_signal() {
    // 执行原始的task4
    existing_task4().await;
    // 发送完成信号
    TASK4_DONE.signal(());
}

#[task]
pub async fn wrapped_task5_signal() {
    // 等待task4完成信号
    TASK4_DONE.wait().await;
    // 执行原始的task5
    existing_task5().await;
}

// 方法3：使用join在一个任务中按顺序执行
#[task]
pub async fn sequential_tasks() {
    // 先执行task4
    existing_task4().await;
    // 再执行task5
    existing_task5().await;
}

// 方法4：使用spawner的返回值（如果任务是一次性的）
#[task]
pub async fn spawn_in_order(spawner: Spawner) {
    // 启动task4并等待其完成
    if let Ok(join_handle) = spawner.spawn(existing_task4()) {
        join_handle.await;
        // task4完成后再启动task5
        let _ = spawner.spawn(existing_task5());
    }
}

// 主函数：展示不同的实现方法
#[embassy_executor::main]
async fn main_ordering_example(spawner: Spawner) {
    info!("=== Task Ordering Examples ===");

    // 方法1：Channel方式
    info!("\n--- Method 1: Using Channel ---");
    let _ = spawner.spawn(wrapped_task4_channel());
    let _ = spawner.spawn(wrapped_task5_channel());
    Timer::after(Duration::from_secs(4)).await;

    // 方法2：Signal方式
    info!("\n--- Method 2: Using Signal ---");
    let _ = spawner.spawn(wrapped_task4_signal());
    let _ = spawner.spawn(wrapped_task5_signal());
    Timer::after(Duration::from_secs(4)).await;

    // 方法3：直接顺序执行
    info!("\n--- Method 3: Direct Sequential Execution ---");
    let _ = spawner.spawn(sequential_tasks());
    Timer::after(Duration::from_secs(4)).await;

    // 方法4：使用join_handle
    info!("\n--- Method 4: Using Join Handle ---");
    let _ = spawner.spawn(spawn_in_order(spawner.clone()));
    Timer::after(Duration::from_secs(4)).await;

    info!("\n=== All Examples Completed ===");
}
