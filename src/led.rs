
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either3, select3};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};

pub static BLINK_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static REVERSE: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(
    mut spedometer_led: Output<'static>,
    mut enable_led: Output<'static>,
) {
    let mut period = Duration::from_millis(2500);
    let mut last_toggle = Instant::now();

    loop {
        let deadline = last_toggle + period;

        match select3(BLINK_MS.wait(), REVERSE.wait(), Timer::at(deadline)).await {
            Either3::First(ms) => period = Duration::from_millis(ms),
            Either3::Second(in_reverse) => {
                if in_reverse {
                    enable_led.set_low()
                } else {
                    enable_led.set_high()
                }
            }
            Either3::Third(_) => {
                spedometer_led.toggle();
                last_toggle = Instant::now();
            }
        }
    }
}