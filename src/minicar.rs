// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use crate::{
    button::ButtonType,
    minicar::{
        CarDirection::{Forward, Reverse},
        Gear::{Gear1, Gear2, Gear3, Gear4},
    },
    //potentiometer::POTENTIOMETER_SIGNAL,
    //temp::TEMPERATURE_SIGNAL,
};
use crate::{
    button::{
        BUTTON_SIGNAL,
        ButtonType::{Accelerate, Brake, Direction},
    },
    led::TOGGLE_TIME_SIGNAL,
    potentiometer::POTENTIOMETER_SIGNAL,
};
use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};

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
    const MAX_VELOCITY: i32 = 100;
    const BREAK_STEP: i32 = 20;

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
    fn velocity_increase(&self) -> i32 {
        let increase = match self.gear {
            Gear1 => 5,
            Gear2 => 10,
            Gear3 => 20,
            Gear4 => 35,
        };
        if self.direction == Reverse {
            return increase * -1;
        }
        return increase;
    }
    fn handle_button(&mut self, button: ButtonType) {
        match button {
            Accelerate => {
                self.velocity = self.velocity + self.velocity_increase();
                self.velocity = self.velocity.clamp(-Self::MAX_VELOCITY, Self::MAX_VELOCITY);
            }
            Direction => {
                if self.direction == Reverse {
                    self.direction = Forward;
                } else {
                    self.direction = Reverse;
                }
            }
            Brake => {
                if self.velocity > 0 {
                    self.velocity = self.velocity - Self::BREAK_STEP;
                    if self.velocity < 0 {
                        self.velocity = 0;
                    }
                } else if self.velocity < 0 {
                    self.velocity = self.velocity + Self::BREAK_STEP;
                    if self.velocity > 0 {
                        self.velocity = 0;
                    }
                }
            }
        }
    }
    fn handle_time(&self) -> Duration {
        let speed: i32 = self.velocity.abs();
        let ms: u64 = match speed {
            s if s < 25 => 2500,
            s if s < 50 => 2000,
            s if s < 75 => 1000,
            _ => 500,
        };
        Duration::from_millis(ms)
    }
}

// TODO: Complete the Minicar implementation!

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();
    let mut last_toggle_time = minicar.handle_time();

    loop {
        match select(BUTTON_SIGNAL.wait(), POTENTIOMETER_SIGNAL.wait()).await {
            Either::First(button) => {
                minicar.handle_button(button);
                info!("{} -> velocity {}", button, minicar.velocity);
            }
            Either::Second(percent) => {
                minicar.set_gear(percent);
            }
        }

        let toggle_time = minicar.handle_time();
        if toggle_time != last_toggle_time {
            TOGGLE_TIME_SIGNAL.signal(toggle_time);
            last_toggle_time = toggle_time;
        }
    }
}
