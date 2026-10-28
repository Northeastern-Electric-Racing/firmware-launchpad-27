// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either3, select3};
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub static TOGGLE_TIME_SIGNAL: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static FAULT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut speedometer_led: Output<'static>) {
    let mut toggle_time = Duration::from_millis(2500);

    loop {
        match select3(
            Timer::after(toggle_time),
            TOGGLE_TIME_SIGNAL.wait(),
            FAULT_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(_) => {
                speedometer_led.toggle();
            }
            Either3::Second(new_time) => {
                toggle_time = new_time;
            }
            Either3::Third(true) => {
                speedometer_led.set_low(); // off
                // Stay here until minicar.rs says the fault cleared
                while FAULT_SIGNAL.wait().await {}
            }
            Either3::Third(false) => {}
        }
    }
}
