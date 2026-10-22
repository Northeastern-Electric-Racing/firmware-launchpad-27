
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
};
use embassy_futures::select::{Either3, select3};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use defmt::info;
use crate::led::{BLINK_PERIOD_SIGNAL, DIRECTION_SIGNAL};
use crate::led::FAULT_SIGNAL;
use crate::temp::TEMPERATURE_SIGNAL;

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
    
    fn update_gear(&mut self, percent: f32) {
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

    fn handle_button(&mut self, button: ButtonType) {
        if self.fault {
        return; }
        match button {
        Direction => {
            self.direction = if self.direction == Forward { Reverse } else { Forward };
        }
        Accelerate => {
            let step = match self.gear {
                Gear1 => 5,
                Gear2 => 10,
                Gear3 => 20,
                Gear4 => 35,
            };
            let signed_step = if self.direction == Forward { step } else { -step };
            self.velocity += signed_step;
            self.velocity = self.velocity.clamp(-100, 100);
        }
        Brake => {
            let speed = self.velocity.abs();
            let new_speed = (speed - 20).max(0);
            self.velocity = new_speed * self.velocity.signum();
        }
    }
}

    fn blink_period_ms(&self) -> u64 {
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

    fn update_temperature(&mut self, celsius: f32) {
    if celsius > 35.0 {
        self.fault = true;
        self.velocity = 0;
    } else if celsius < 35.0 {
        self.fault = false;
    }
}
}

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();

    loop {
        match select3(
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
            TEMPERATURE_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(button) => {
                let old_direction = minicar.direction;
                minicar.handle_button(button);
                if minicar.direction != old_direction {
                    DIRECTION_SIGNAL.signal(minicar.direction == Reverse);
                }
                BLINK_PERIOD_SIGNAL.signal(minicar.blink_period_ms());
                info!("velocity: {}", minicar.velocity);
            }
            Either3::Second(percent) => {
                minicar.update_gear(percent);
            }
            Either3::Third(celsius) => {
                let was_fault = minicar.fault;
                minicar.update_temperature(celsius);
                if minicar.fault != was_fault {
                    FAULT_SIGNAL.signal(minicar.fault);
                    BLINK_PERIOD_SIGNAL.signal(minicar.blink_period_ms());
                    info!("fault: {}, velocity: {}", minicar.fault, minicar.velocity);
                }
            }
        }
    }
}