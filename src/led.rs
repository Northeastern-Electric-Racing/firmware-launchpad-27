// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub static TOGGLE_TIME_SIGNAL: Signal<CriticalSectionRawMutex, Duration> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut speedometer_led: Output<'static>) {
    let mut toggle_time = Duration::from_millis(2500);

    loop {
        match select(Timer::after(toggle_time), TOGGLE_TIME_SIGNAL.wait()).await {
            Either::First(_) => {
                speedometer_led.toggle();
            }
            Either::Second(new_time) => {
                toggle_time = new_time;
            }
        }
    }
}
