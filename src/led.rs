// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use crate::minicar::{FAULT_SIGNAL, SPEEDOMETER_SIGNAL};
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_stm32::gpio::Output;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    // TODO: LED Task
    let mut toggle_time: u64 = 2500;
    let mut faulted = false;

    enable_led.set_high();

    loop {
        if faulted {
            match select(SPEEDOMETER_SIGNAL.wait(), FAULT_SIGNAL.wait()).await {
                Either::First(new_time) => {
                    toggle_time = new_time;
                }
                Either::Second(fault) => {
                    faulted = fault;
                    if !faulted {
                        enable_led.set_high();
                    }
                }
            }
        } else {
            match select3(
                Timer::after_millis(toggle_time),
                SPEEDOMETER_SIGNAL.wait(),
                FAULT_SIGNAL.wait(),
            )
            .await
            {
                Either3::First(_) => {
                    spedometer_led.toggle();
                }
                Either3::Second(new_time) => {
                    toggle_time = new_time;
                }
                Either3::Third(fault) => {
                    faulted = fault;
                    if faulted {
                        spedometer_led.set_low();
                        enable_led.set_low();
                    }
                }
            }
        }
    }
}
