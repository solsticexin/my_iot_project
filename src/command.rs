use crate::config::{COMMAND_CHANNEL, UART_TX_CHANNEL};
use crate::protocol::{ActuatorFeedback, ActuatorTag, CommandAck, ControlCommand, TxMessage};
use embassy_executor::task;
use embassy_stm32::gpio::{Level, Speed};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Timer};

// 定义每种执行器的命令通道
// 这些通道不需要全局，因为它们只在 main 中初始化并传递，或者为了方便，我们在 command.rs 内部定义辅助结构
// 为了简洁，这里我们可以定义一个 Channel 类型别名
pub type ActuatorChannel = Channel<CriticalSectionRawMutex, ControlCommand, 2>;

#[task]
pub async fn command_task(
    fan_sender: Sender<'static, CriticalSectionRawMutex, ControlCommand, 2>,
    pump_sender: Sender<'static, CriticalSectionRawMutex, ControlCommand, 2>,
    light_sender: Sender<'static, CriticalSectionRawMutex, ControlCommand, 2>,
    buzzer_sender: Sender<'static, CriticalSectionRawMutex, ControlCommand, 2>,
) {
    let receiver = COMMAND_CHANNEL.receiver();
    let tx_sender = UART_TX_CHANNEL.sender();

    loop {
        let cmd = receiver.receive().await;

        // 1. 发送 ACK
        // 这里的 ACK 表示"收到并分发成功"，并不代表物理动作完成，但也足够了
        // 如果需要执行后 ACK，需要 ActuatorFeedback
        let ack = CommandAck {
            actuator: cmd.actuator,
            success: true,
        };
        tx_sender.send(TxMessage::Ack(ack)).await;

        // 2. 分发给具体的 Actuator Task
        let target_sender = match cmd.actuator {
            ActuatorTag::Fan => &fan_sender,
            ActuatorTag::Pump => &pump_sender,
            ActuatorTag::Light => &light_sender,
            ActuatorTag::Buzzer => &buzzer_sender,
        };

        target_sender.send(cmd).await;
    }
}

// 通用的执行器任务
// active_level: true 表示高电平触发/打开，false 表示低电平触发/打开
use embassy_sync::channel::Receiver;

#[task(pool_size = 4)]
pub async fn actuator_task(
    mut flex: embassy_stm32::gpio::Flex<'static>,
    receiver: Receiver<'static, CriticalSectionRawMutex, ControlCommand, 2>,
    active_high: bool,
) {
    // 初始状态 OFF
    // 如果 active_high，OFF 是 Low
    // 如果 !active_high (低触), OFF 是 High
    let initial_level = if active_high { Level::Low } else { Level::High };
    flex.set_as_output(Speed::Low);
    flex.set_level(initial_level);

    let tx_sender = UART_TX_CHANNEL.sender();

    loop {
        let cmd = receiver.receive().await;

        // 执行动作
        let target_level = if cmd.state {
            if active_high { Level::High } else { Level::Low }
        } else {
            if active_high { Level::Low } else { Level::High }
        };

        flex.set_level(target_level);

        // 上报状态
        let feedback = ActuatorFeedback {
            actuator: cmd.actuator,
            state: cmd.state,
        };
        tx_sender.send(TxMessage::Actuator(feedback)).await;

        // 处理 Pulse
        // 如果 state = true 且 duration > 0
        if cmd.state && cmd.duration_ms > 0 {
            // 等待时间
            Timer::after(Duration::from_millis(cmd.duration_ms as u64)).await;

            // 恢复 OFF
            let off_level = if active_high { Level::Low } else { Level::High };
            flex.set_level(off_level);

            // 上报状态 OFF
            let feedback_off = ActuatorFeedback {
                actuator: cmd.actuator,
                state: false,
            };
            tx_sender.send(TxMessage::Actuator(feedback_off)).await;
        }
    }
}
