use embassy_stm32::gpio::{Level, Output};
use embassy_time::{Duration, Timer};

pub struct  Actuator<'d>{
    water:Output<'d>,
    fan:Output<'d>,
    light:Output<'d>,
    buzzer:Output<'d>,
}
impl<'d> Actuator<'d> {
    pub fn new(
        mut water:Output<'d>,
        mut fan:Output<'d>,
        mut light:Output<'d>,
        mut buzzer:Output<'d>,
    )->Self{
        water.set_level(Level::High);
        fan.set_level(Level::High);
        light.set_level(Level::High);
        buzzer.set_level(Level::High);
        Self{water,fan,light,buzzer}
    }
    ///执行器低电平触发
    pub fn set_on(&mut self,act: Act)->Ack{
        match act {
            Act::Water=> { self.water.set_level(Level::Low);Ack::On(Act::Water) },
            Act::Fan=> { self.fan.set_level(Level::Low);Ack::On(Act::Fan)},
            Act::Light=> { self.light.set_level(Level::Low);Ack::On(Act::Light) },
            Act::Buzzer=> { self.buzzer.set_level(Level::Low);Ack::On(Act::Buzzer) },
        }
    }
    pub fn set_off(&mut self,act: Act)->Ack{
        match act {
            Act::Water=> { self.water.set_level(Level::High);Ack::Off(Act::Water) },
            Act::Fan=> { self.fan.set_level(Level::High);Ack::Off(Act::Fan)},
            Act::Light=> { self.light.set_level(Level::High);Ack::Off(Act::Light) },
            Act::Buzzer=> { self.buzzer.set_level(Level::High);Ack::Off(Act::Buzzer) },
        }
    }
    pub async fn set_pulse(&mut self,act: Act,time:u64)->Ack{
        match act {
            Act::Water=> {
                self.water.set_level(Level::High);
                Timer::after(Duration::from_secs(time)).await;
                Ack::Pulse(Act::Water,time)
            },
            Act::Fan=> {
                self.fan.set_level(Level::High);
                Timer::after(Duration::from_secs(time)).await;
                Ack::Pulse(Act::Fan,time)},
            Act::Light=> {
                self.light.set_level(Level::High);
                Timer::after(Duration::from_secs(time)).await;
                Ack::Pulse(Act::Light,time)
            },
            Act::Buzzer=> {
                self.buzzer.set_level(Level::High);
                Timer::after(Duration::from_secs(time)).await;
                Ack::Pulse(Act::Buzzer,time)
            },
        }
    }
}
///执行器
pub enum Act{
    Water,
    Fan,
    Light,
    Buzzer
}
pub enum Ack{
    On(Act),
    Off(Act),
    Pulse(Act,u64),
}