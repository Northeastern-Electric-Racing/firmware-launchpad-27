
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use crate::{button::{
    BUTTON_SIGNAL,
    ButtonType::*,
}, led::LightStates::*};
use crate::{
    minicar::{
        CarDirection::{Forward, Reverse},
        Gear::{Gear1, Gear2, Gear3, Gear4},
    },
    potentiometer::POTENTIOMETER_SIGNAL,
    //temp::TEMPERATURE_SIGNAL,
    led::LIGHT_SIGNAL,
};
use defmt::warn;
use embassy_futures::select::{Either, select};


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

    fn accelerate(&mut self) {
        match self.gear {
            Gear1 => {
                self.velocity += 5;
            }
            Gear2 => {
                self.velocity += 10;
            }
            Gear3 => {
                self.velocity += 20;
            }
            Gear4 => {
                self.velocity += 35;
            }
        }
        if self.velocity > 100 {
            self.velocity = 100;
        }
    }

    fn brake(&mut self) {
        self.velocity -= 20;
        if self.velocity < 0 {
            self.velocity = 0;
        }
    }

    fn change_gear(&mut self, pot_value: f32) {
        match pot_value {
            p if p < 0.25 => {
                self.gear = Gear1;
            }
            p if p < 0.5 => {
                self.gear = Gear2;
            }
            p if p < 0.75 => {
                self.gear = Gear3;
            }
            p if p <= 1.0 => {
                self.gear = Gear4;
            }
            p => {
                warn!("Improper potentiometer value {}", p);
            }
        }
    }

    fn change_direction(&mut self) {
        if self.direction == Forward {
            self.direction = Reverse;
        }
        else {
            self.direction = Forward;
        }
    }

    fn update_light_signals(&mut self) {
        match self.fault {
            true => {
                LIGHT_SIGNAL.signal(Off);
            }
            false => {
                match self.direction {
                    Forward => {
                        match self.velocity {
                            0 => {
                                LIGHT_SIGNAL.signal(Stationary);
                            }
                            v if v < 25 => {
                                LIGHT_SIGNAL.signal(Speed1);
                            }
                            v if v < 50 => {
                                LIGHT_SIGNAL.signal(Speed2);
                            }
                            v if v < 75 => {
                                LIGHT_SIGNAL.signal(Speed3);
                            }
                            v if v <= 100 => {
                                LIGHT_SIGNAL.signal(Speed4);
                            }
                            v => {
                                warn!("Improper velocity value {}", v);
                            }
                        }
                    }
                    Reverse => {
                        match self.velocity {
                            0 => {
                                LIGHT_SIGNAL.signal(StationaryReverse);
                            }
                            v if v < 25 => {
                                LIGHT_SIGNAL.signal(Speed1Reverse);
                            }
                            v if v < 50 => {
                                LIGHT_SIGNAL.signal(Speed2Reverse);
                            }
                            v if v < 75 => {
                                LIGHT_SIGNAL.signal(Speed3Reverse);
                            }
                            v if v <= 100 => {
                                LIGHT_SIGNAL.signal(Speed4Reverse);
                            }
                            v => {
                                warn!("Improper velocity value {}", v);
                            }
                        }
                    }
                }
            }
        }
    }
    
}

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();
    minicar.update_light_signals();

    loop {
        match select (
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
        ).await {
            Either::First(button_signal) => {
                match button_signal {
                    Accelerate => {
                        minicar.accelerate();
                    }
                    Brake => {
                        minicar.brake();
                    }
                    Direction => {
                        minicar.change_direction();
                    }
                }
            }
            Either::Second(potentiometer_signal) => {
                minicar.change_gear(potentiometer_signal);
            }
        }
        minicar.update_light_signals();
    }
}
