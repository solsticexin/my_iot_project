use embassy_stm32::gpio::{Level, Output};

///增加静态bool用来打断命令该命令只在 use crate::uart::uart_rx;中修改，

///执行器执行完命令后执行的状态：0表示关1表示开
pub struct ActuatorStatus {
    pub water: u8,
    pub fan: u8,
    pub light: u8,
    pub buzzer: u8,
}

///执行器全局唯一，状态唯一
///执行器状态的修改只在set_on函数中修改,所以不需要互斥锁
pub static mut ACTUATOR_STATUS: ActuatorStatus = ActuatorStatus {
    water: 0,
    fan: 0,
    light: 0,
    buzzer: 0,
};

///执行器低电平触发
pub struct Actuator<'d> {
    water: Output<'d>,
    fan: Output<'d>,
    light: Output<'d>,
    buzzer: Output<'d>,
}
impl<'d> Actuator<'d> {
    pub fn new(
        mut water: Output<'d>,
        mut fan: Output<'d>,
        mut light: Output<'d>,
        mut buzzer: Output<'d>,
    ) -> Self {
        water.set_level(Level::High);
        fan.set_level(Level::High);
        light.set_level(Level::High);
        buzzer.set_level(Level::High);
        Self {
            water,
            fan,
            light,
            buzzer,
        }
    }

    ///使用async只是为了保持封装一致
    pub async fn set_on(&mut self, act: Act) {
        match act {
            Act::Water => {
                self.water.set_level(Level::Low);
                unsafe {
                    ACTUATOR_STATUS.water = 1;
                }
            }
            Act::Fan => {
                self.fan.set_level(Level::Low);
                unsafe {
                    ACTUATOR_STATUS.fan = 1;
                }
            }
            Act::Light => {
                self.light.set_level(Level::Low);
                unsafe {
                    ACTUATOR_STATUS.light = 1;
                }
            }
            Act::Buzzer => {
                self.buzzer.set_level(Level::Low);
                unsafe {
                    ACTUATOR_STATUS.buzzer = 1;
                }
            }
        }
    }

    ///使用async只是为了保持封装一致
    pub async fn set_off(&mut self, act: Act) {
        match act {
            Act::Water => {
                self.water.set_level(Level::High);
                unsafe {
                    ACTUATOR_STATUS.water = 0;
                }
            }
            Act::Fan => {
                self.fan.set_level(Level::High);
                unsafe {
                    ACTUATOR_STATUS.fan = 0;
                }
            }
            Act::Light => {
                self.light.set_level(Level::High);
                unsafe {
                    ACTUATOR_STATUS.light = 0;
                }
            }
            Act::Buzzer => {
                self.buzzer.set_level(Level::High);
                unsafe {
                    ACTUATOR_STATUS.buzzer = 0;
                }
            }
        }
    }
}
///执行器
#[derive(Copy, Clone)]
pub enum Act {
    Water,
    Fan,
    Light,
    Buzzer,
}
#[derive(Copy, Clone)]
pub enum Switch {
    On,
    Off,
    Pulse(u64),
}
pub enum Ack {
    On(Act),
    Off(Act),
    Pulse(Act, u64),
}
impl Ack {
    pub fn analysis(&self) -> (Act, Switch) {
        match self {
            Ack::On(act) => (*act, Switch::On),
            Ack::Off(act) => (*act, Switch::Off),
            Ack::Pulse(act, time) => (*act, Switch::Pulse(*time)),
        }
    }
}
