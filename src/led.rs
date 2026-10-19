
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either3, select3};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Ticker, Timer};

pub static BLINK_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static REVERSE: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(
    mut spedometer_led: Output<'static>,
    mut enable_led: Output<'static>,
) {
    let mut ticker = Ticker::every(Duration::from_millis(2500));

    loop {

        match select3(BLINK_MS.wait(), REVERSE.wait(), ticker.next()).await {
            Either3::First(ms) => ticker = Ticker::every(Duration::from_millis(ms.max(20))),
            Either3::Second(in_reverse) => {
                if in_reverse {
                    enable_led.set_low()
                } else {
                    enable_led.set_high()
                }
            }
            Either3::Third(_) => {
                spedometer_led.toggle();
            }
        }
    }
}