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
use embassy_futures::select::{Either3, Either4, select3, select4};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

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

    // Velocity gained (m/s) per Drive button press in the current gear
    fn acceleration_step(&self) -> i32 {
        match self.gear {
            Gear1 => 5,
            Gear2 => 10,
            Gear3 => 20,
            Gear4 => 35,
        }
    }

    fn accelerate(&mut self) {
        if self.fault {
            return;
        }
        // In reverse the increase is negated so the car moves the opposite way
        let step = match self.direction {
            Forward => self.acceleration_step(),
            Reverse => -self.acceleration_step(),
        };
        self.velocity = (self.velocity + step).clamp(-MAX_VELOCITY, MAX_VELOCITY);
    }

    // Slow the car toward 0 regardless of which way it is moving
    fn brake(&mut self) {
        self.velocity = if self.velocity > 0 {
            (self.velocity - BRAKE_STEP).max(0)
        } else {
            (self.velocity + BRAKE_STEP).min(0)
        };
    }

    fn reverse(&mut self) {
        self.direction = match self.direction {
            Forward => Reverse,
            Reverse => Forward,
        };
    }

    // Map the potentiometer rotation percentage (0.0 - 100.0) onto a gear
    fn set_gear_from_pot(&mut self, pot: f32) {
        self.gear = match pot {
            p if p < 25.0 => Gear1,
            p if p < 50.0 => Gear2,
            p if p < 75.0 => Gear3,
            _ => Gear4,
        };
    }

    // A fault (e.g. over-temperature) immediately stops the car
    fn set_fault(&mut self, fault: bool) {
        self.fault = fault;
        if fault {
            self.velocity = 0;
        }
    }

    // Signed velocity: negative when moving in reverse
    fn speed(&self) -> i32 {
        self.velocity
    }
}

const BRAKE_STEP: i32 = 20;
const MAX_VELOCITY: i32 = 100;
const MAX_TEMPERATURE: f32 = 35.0;

// Latest car speed (m/s, always positive) for the LED task to display
pub static SPEED_SIGNAL: Signal<CriticalSectionRawMutex, i32> = Signal::new();

// Whether the car is currently faulted (overheated), for the LED task
pub static FAULT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();

    loop {
        // Wait for whichever input arrives first: a button press, a pot reading or a temperature reading
        match select3(
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
            TEMPERATURE_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(button) => match button {
                Accelerate => minicar.accelerate(),
                Brake => minicar.brake(),
                Direction => minicar.reverse(),
            },
            Either3::Second(pot) => minicar.set_gear_from_pot(pot),
            // Fault while overheated; clears automatically once the temperature drops back down
            Either3::Third(temp) => {
                let faulted = temp > MAX_TEMPERATURE;
                minicar.set_fault(faulted);
                FAULT_SIGNAL.signal(faulted);
            }
        }

        SPEED_SIGNAL.signal(minicar.velocity.abs());
    }
}
