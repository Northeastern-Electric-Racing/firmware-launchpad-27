// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use crate::button::{
    BUTTON_SIGNAL,
    ButtonType::{Accelerate, Brake, Direction},
};
use crate::{
    button::ButtonType,
    minicar::{
        CarDirection::{Forward, Reverse},
        Gear::{Gear1, Gear2, Gear3, Gear4},
    },
    potentiometer::POTENTIOMETER_SIGNAL,
    temp::TEMPERATURE_SIGNAL,
};
use defmt::info;
use embassy_futures::select::{Either4, select4};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

pub static SPEEDOMETER_SIGNAL: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static FAULT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[derive(PartialEq, Eq, Clone, Copy)]
enum CarDirection {
    Forward,
    Reverse,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Gear {
    Gear1,
    Gear2,
    Gear3,
    Gear4,
}
pub struct Minicar {
    velocity: i32,
    gear: Gear,
    direction: CarDirection,
    fault: bool,
}

impl Minicar {
    fn new() -> Self {
        Self {
            velocity: 0,
            gear: Gear1,
            direction: Forward,
            fault: false,
        }
    }

    // TODO: Complete the Minicar implementation!
    fn set_gear(&mut self, percent: f32) {
        if percent < 25.0 {
            self.gear = Gear1;
        } else if percent < 50.0 {
            self.gear = Gear2;
        } else if percent < 75.0 {
            self.gear = Gear3;
        } else {
            self.gear = Gear4;
        }
    }

    fn get_increase(&self) -> i32 {
        match self.gear {
            Gear1 => 5,
            Gear2 => 10,
            Gear3 => 20,
            Gear4 => 35,
        }
    }

    fn button_pressed(&mut self, button: ButtonType) {
        if self.fault {
            return;
        }

        match button {
            Accelerate => {
                if self.direction == Forward {
                    self.velocity += self.get_increase();
                } else {
                    self.velocity -= self.get_increase();
                }

                if self.velocity > 100 {
                    self.velocity = 100;
                }
                if self.velocity < -100 {
                    self.velocity = -100;
                }
            }
            Brake => {
                if self.velocity > 0 {
                    self.velocity -= 20;
                    if self.velocity < 0 {
                        self.velocity = 0;
                    }
                } else if self.velocity < 0 {
                    self.velocity += 20;
                    if self.velocity > 0 {
                        self.velocity = 0;
                    }
                }
            }
            Direction => {
                if self.direction == Forward {
                    self.direction = Reverse;
                } else {
                    self.direction = Forward;
                }
            }
        }
    }

    fn check_temp(&mut self, temp: f32) {
        if temp > 35.0 {
            self.fault = true;
            self.velocity = 0;
        } else if temp < 35.0 {
            self.fault = false;
        }
    }

    fn get_toggle_time(&self) -> u64 {
        let speed = self.velocity.abs();

        if speed < 25 {
            2500
        } else if speed < 50 {
            2000
        } else if speed < 75 {
            1000
        } else {
            500
        }
    }
}

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();
    let mut toggle_time = minicar.get_toggle_time();

    loop {
        // TODO: Complete State Machine Task
        let event = select4(
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
            TEMPERATURE_SIGNAL.wait(),
            Timer::after_millis(1000),
        )
        .await;

        match event {
            Either4::First(button) => {
                minicar.button_pressed(button);
                info!("velocity: {}", minicar.velocity);
            }
            Either4::Second(percent) => {
                minicar.set_gear(percent);
            }
            Either4::Third(temp) => {
                let was_faulted = minicar.fault;
                minicar.check_temp(temp);

                if minicar.fault != was_faulted {
                    FAULT_SIGNAL.signal(minicar.fault);
                    info!("fault: {}, temp: {}", minicar.fault, temp);
                }
            }
            Either4::Fourth(_) => {
                info!("velocity: {}, fault: {}", minicar.velocity, minicar.fault);
            }
        }

        if minicar.get_toggle_time() != toggle_time {
            toggle_time = minicar.get_toggle_time();
            SPEEDOMETER_SIGNAL.signal(toggle_time);
        }
    }
}
