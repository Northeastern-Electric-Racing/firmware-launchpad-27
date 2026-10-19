
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***
use crate::led::{BLINK_MS,REVERSE};
use crate::potentiometer::PEDAL_PCT;
use crate::temp::FAULT;
use crate::{button::{
    BUTTONS,Button
}};
use defmt::info;
use embassy_futures::select::{Either3, select3};

const MAX_SPEED: i32 = 100;
const BRAKE_STEP: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
enum Gear {
    Gear1,
    Gear2,
    Gear3,
    Gear4,
}


impl Gear {
    fn from_percent(pct: u32) -> Self {
      match pct {
            0..=24 => Gear::Gear1,
            25..=49 => Gear::Gear2,
            50..=74 => Gear::Gear3,
            _ => Gear::Gear4,
        }
    }
    fn step(self) -> i32 {
        match self {
            Gear::Gear1 => 5,
            Gear::Gear2 => 10,
            Gear::Gear3 => 20,
            Gear::Gear4 => 35,
        }
    }
}

pub struct MiniCar {
    velocity: i32,
    gear: Gear,
    reverse: bool,
}
 
impl MiniCar {
    fn new() -> Self {
        Self {
            velocity: 0,
            gear: Gear::Gear1,
            reverse: false,
        }
    }
    fn on_button(&mut self, button: Button) {
        match button {
            Button::Drive => {
                let step = self.gear.step();
                let delta = if self.reverse {-step} else {step};
                self.velocity = (self.velocity + delta).clamp(-MAX_SPEED, MAX_SPEED);
            }
            Button::Brake => {
                self.velocity = 
                if self.velocity > 0 {
                    (self.velocity - BRAKE_STEP).max(0)
                } 
                else {
                    (self.velocity + BRAKE_STEP).min(0)
                };
            }
            Button::Reverse => {
                self.reverse = !self.reverse;
                REVERSE.signal(self.reverse);
            }
        }
    }
    fn blink_ms(&self) -> u64 {
        match self.velocity.abs() {
            0..=24 => 2500,
            25..=49 => 2000,
            50..=74 => 1000,
            _ => 500,
        }
    }
}

#[embassy_executor::task]
pub async fn car_task() {
    let mut car = MiniCar::new();
    BLINK_MS.signal(car.blink_ms());
    let mut fault = FAULT.receiver().unwrap();
    let mut faulted = false;
 
    loop {
        match select3(fault.changed(), BUTTONS.receive(), PEDAL_PCT.wait()).await {
            Either3::First(f) => {
                faulted = f;
                car.velocity = 0;
            }
            Either3::Second(button) => {
                if !faulted {
                    car.on_button(button);
                }
            },
            Either3::Third(pct) => car.gear = Gear::from_percent(pct),
        }
        BLINK_MS.signal(car.blink_ms());
        info!(
            "gear: {}, velocity: {} m/s, reverse: {}",
            car.gear, car.velocity, car.reverse
        );
    }
}