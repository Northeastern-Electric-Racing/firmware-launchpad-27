
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use crate::button::{
    BUTTON_SIGNAL,
    ButtonType::{Accelerate, Brake, Direction},
};
use crate::potentiometer::POTENTIOMETER_SIGNAL;
use crate::led::{ENABLE_SIGNAL, SPEEDOMETER_SIGNAL};

use self::CarDirection::{Forward, Reverse};
use self::Gear::{Gear1, Gear2, Gear3, Gear4};

use embassy_futures::select::{select, Either};

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

    fn set_gear(&mut self, percentage: f32) {
        if percentage < 25.0 {
            self.gear = Gear1;
        } else if percentage < 50.0 {
            self.gear = Gear2;
        } else if percentage < 75.0 {
            self.gear = Gear3;
        } else {
            self.gear = Gear4;
        }
    }

    fn drive(&mut self) {
        let velocity_change = match self.gear {
            Gear1 => 5,
            Gear2 => 10,
            Gear3 => 20,
            Gear4 => 35
        };

        match self.direction {
            Forward => {
                self.velocity += velocity_change;
            }

            Reverse => {
                self.velocity -= velocity_change;
            }
        }

        if self.velocity > 100 {
            self.velocity = 100;
        }

        if self.velocity < -100 {
            self.velocity = -100;
        }
    }

    fn brake(&mut self) {
        if self.velocity > 0 {
            self.velocity -=20;

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

    fn reverse(&mut self) {
        match self.direction {
            Forward => {
                self.direction = Reverse;
            }

            Reverse => {
                self.direction = Forward;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn state_machine_task() {
    let mut minicar = Minicar::new();

    SPEEDOMETER_SIGNAL.signal(minicar.velocity);

    loop {
        let event = select(
            BUTTON_SIGNAL.wait(),
            POTENTIOMETER_SIGNAL.wait(),
        ).await;

        match event {
            Either::First(button) => {
                match button {
                    Accelerate => {
                        minicar.drive();
                        ENABLE_SIGNAL.signal(true);
                    }

                    Brake => {
                        minicar.brake();
                    }

                    Direction => {
                        minicar.reverse();
                    }
                }

                SPEEDOMETER_SIGNAL.signal(minicar.velocity);
            }

            Either::Second(percentage) => {
                minicar.set_gear(percentage);
            }
        }
    }
}
