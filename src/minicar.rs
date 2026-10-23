use crate::button::{
    BUTTON_SIGNAL,
    ButtonType::{Accelerate, Brake, Direction},
};
use crate::{
    button::ButtonType,
    led::SPEEDOMETER_SIGNAL,
    minicar::{
        CarDirection::{Forward, Reverse},
        Gear::{Gear1, Gear2, Gear3, Gear4},
    },
    potentiometer::POTENTIOMETER_SIGNAL,
    temp::TEMPERATURE_SIGNAL,
};
use defmt::{info, warn};
use embassy_futures::select::{Either3, select3};

const MAX_VELOCITY: i32 = 100;
const BRAKE_DECREASE: i32 = 20;
const MAX_TEMPERATURE_C: f32 = 35.0;

#[derive(PartialEq, Eq, Clone, Copy, defmt::Format)]
enum CarDirection {
    Forward,
    Reverse,
}

#[derive(PartialEq, Eq, Clone, Copy, defmt::Format)]
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

    fn update_gear(&mut self, pedal_percent: f32) -> Option<Gear> {
        let new_gear = if pedal_percent < 25.0 {
            Gear1
        } else if pedal_percent < 50.0 {
            Gear2
        } else if pedal_percent < 75.0 {
            Gear3
        } else {
            Gear4
        };

        if new_gear == self.gear {
            return None;
        }

        self.gear = new_gear;
        Some(new_gear)
    }

    fn velocity_increase(&self) -> i32 {
        let increase = match self.gear {
            Gear1 => 5,
            Gear2 => 10,
            Gear3 => 20,
            Gear4 => 35,
        };

        match self.direction {
            Forward => increase,
            Reverse => -increase,
        }
    }

    fn accelerate(&mut self) {
        self.velocity = (self.velocity + self.velocity_increase()).clamp(-MAX_VELOCITY, MAX_VELOCITY);
    }

    fn brake(&mut self) {
        if self.velocity > 0 {
            self.velocity = (self.velocity - BRAKE_DECREASE).max(0);
        } else {
            self.velocity = (self.velocity + BRAKE_DECREASE).min(0);
        }
    }

    fn reverse_direction(&mut self) {
        self.direction = match self.direction {
            Forward => Reverse,
            Reverse => Forward,
        };
    }

    fn speed(&self) -> i32 {
        self.velocity.abs()
    }

    fn speedometer_toggle_ms(&self) -> u64 {
        match self.speed() {
            0..=24 => 2500,
            25..=49 => 2000,
            50..=74 => 1000,
            _ => 500,
        }
    }

    fn speedometer_command(&self) -> Option<u64> {
        if self.fault {
            None
        } else {
            Some(self.speedometer_toggle_ms())
        }
    }

    fn update_fault(&mut self, temperature_c: f32) -> Option<bool> {
        let new_fault = if temperature_c > MAX_TEMPERATURE_C {
            true
        } else if temperature_c < MAX_TEMPERATURE_C {
            false
        } else {
            self.fault
        };

        if new_fault == self.fault {
            return None;
        }

        self.fault = new_fault;
        if new_fault {
            self.velocity = 0;
        }
        Some(new_fault)
    }

    fn handle_button(&mut self, button: ButtonType) {
        match button {
            Accelerate => self.accelerate(),
            Brake => self.brake(),
            Direction => self.reverse_direction(),
        }
    }
}

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();
    SPEEDOMETER_SIGNAL.signal(minicar.speedometer_command());

    loop {
        match select3(
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
            TEMPERATURE_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(button) => {
                if minicar.fault {
                    warn!("button ignored: car is faulted");
                    continue;
                }

                minicar.handle_button(button);
                SPEEDOMETER_SIGNAL.signal(minicar.speedometer_command());
                info!(
                    "velocity = {} m/s, gear = {}, direction = {}",
                    minicar.velocity, minicar.gear, minicar.direction
                );
            }
            Either3::Second(pedal_percent) => {
                if let Some(gear) = minicar.update_gear(pedal_percent) {
                    info!("gear = {} ({}%)", gear, pedal_percent);
                }
            }
            Either3::Third(temperature_c) => {
                if let Some(fault) = minicar.update_fault(temperature_c) {
                    SPEEDOMETER_SIGNAL.signal(minicar.speedometer_command());
                    if fault {
                        warn!("FAULT: {} C is above {} C, velocity set to 0", temperature_c, MAX_TEMPERATURE_C);
                    } else {
                        info!("fault cleared: {} C", temperature_c);
                    }
                }
            }
        }
    }
}
