
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;


pub enum LightStates {
    Off,
    Stationary,
    Speed1,
    Speed2,
    Speed3,
    Speed4,
}

pub static LIGHT_SIGNAL: Signal<CriticalSectionRawMutex, LightStates> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    // TODO: LED Task
    let mut state: LightStates = LightStates::Off;
    let mut counter: u8 = 0;

    loop {
        match select(
            LIGHT_SIGNAL.wait(),
            Timer::after_millis(500),
        ).await {
            Either::First(sig) => {
                state = sig;
            }
            Either::Second(_) => {
                match state {
                    LightStates::Off=> {
                        enable_led.set_low();
                        spedometer_led.set_low();
                    }
                    LightStates::Stationary=> {
                        enable_led.set_high();
                        spedometer_led.set_low();
                    }
                    LightStates::Speed1=> {
                        enable_led.set_high();
                        if counter > 5 {
                            spedometer_led.set_high();
                            counter = 0;
                        }
                        else {
                            spedometer_led.set_low();
                        }
                    }
                    LightStates::Speed2=> {
                        enable_led.set_high();
                        if counter > 4 {
                            spedometer_led.set_high();
                            counter = 0;
                        }
                        else {
                            spedometer_led.set_low();
                        }
                    }
                    LightStates::Speed3=> {
                        enable_led.set_high();
                        if counter > 2 {
                            spedometer_led.set_high();
                            counter = 0;
                        }
                        else {
                            spedometer_led.set_low();
                        }
                    }
                    LightStates::Speed4=> {
                        enable_led.set_high();
                        if counter >  1 {
                            spedometer_led.set_high();
                            counter = 0;
                        }
                        else {
                            spedometer_led.set_low();
                        }
                    }
                }
                counter = counter + 1;
            }
        }
    }
}
