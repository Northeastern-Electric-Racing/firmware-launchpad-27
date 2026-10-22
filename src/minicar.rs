
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***
use crate::{
    button::ButtonType::{self, Accelerate, Brake, Direction},
    button::BUTTON_SIGNAL,
    minicar::{
        CarDirection::{Forward, Reverse},
        Gear::{Gear1, Gear2, Gear3, Gear4},
    },
    potentiometer::POTENTIOMETER_SIGNAL,
    temp::TEMPERATURE_SIGNAL,
};
use embassy_futures::select::{Either4, select4};
use embassy_time::Timer;

use crate::led::{LED_SIGNAL, LedState};


const MAX_TEMP_C: f32 = 60.0;
const SPEED_STEP: i32 = 1;

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

impl Gear {
    fn max_speed(self) -> i32 {
        match self { Gear1 => 5, Gear2 => 10, Gear3 => 15, Gear4 => 20 }
    }
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
    fn handle_button(&mut self, button: ButtonType) {
        if self.fault { return; }
            match button {
                Accelerate => self.velocity = (self.velocity + SPEED_STEP).min(self.gear.max_speed()),
                Brake => self.velocity = (self.velocity - SPEED_STEP).max(0),
                Direction => {
                    if self.velocity == 0 {
                         self.direction = if self.direction == Forward { Reverse } else { Forward };
                    }
                }  
            }
    }
    fn set_gear_from_pot(&mut self, f: f32) {
    self.gear = if f < 0.25 { Gear1 } else if f < 0.5 { Gear2 } else if f < 0.75 { Gear3 } else { Gear4 };
    self.velocity = self.velocity.min(self.gear.max_speed());
}
    fn coast(&mut self) {
         self.velocity = (self.velocity - 1).max(0); 
    }
    fn led_state(&self) -> LedState {
    LedState { enabled: !self.fault, velocity: self.velocity }
}
    fn check_temp(&mut self, temp: f32) {
        self.fault = temp > MAX_TEMP_C;
        if self.fault { self.velocity = 0; }
    }
}


#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();

    loop {
        // TODO: Complete State Machine Task
        match select4(
             BUTTON_SIGNAL.wait(),
             POTENTIOMETER_SIGNAL.wait(),
             TEMPERATURE_SIGNAL.wait(),
             Timer::after_millis(500),
        ).await {    
        Either4::First(b) => minicar.handle_button(b),
        Either4::Second(f) => minicar.set_gear_from_pot(f),
        Either4::Third(c) => minicar.check_temp(c),
        Either4::Fourth(_) => minicar.coast(),
        }
         LED_SIGNAL.signal(minicar.led_state());
    }
}
   
