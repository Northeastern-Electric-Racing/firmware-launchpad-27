
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***
use embassy_stm32::gpio::Output;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_futures::select::{select3, Either3};
use embassy_time::Timer;

// LED toggle interval in milliseconds
pub static ENABLE_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static SPEEDOMETER_SIGNAL: Signal<CriticalSectionRawMutex, i32> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut speedometer_led: Output<'static>) {
    // TODO: LED Task

    let mut toggle_time = 2500;

    loop {
        let event = select3(
            ENABLE_SIGNAL.wait(),
            SPEEDOMETER_SIGNAL.wait(),
            Timer::after_millis(toggle_time),
        ).await;

        match event {
            Either3::First(_) => {
                enable_led.toggle();
            }

            Either3::Second(velocity) => {
                let speed = velocity.abs();

                toggle_time = if speed < 25 {
                    2500
                } else if speed < 50 {
                    2000
                } else if speed < 75 {
                    1000
                } else {
                    500
                };
            }

            Either3::Third(_) => {
                speedometer_led.toggle();
            }
        }
    }
}
