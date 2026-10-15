// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

pub static BLINK_PERIOD_SIGNAL: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static DIRECTION_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    let mut period_ms: u64 = 2500;

    loop {
        match select(Timer::after_millis(period_ms), DIRECTION_SIGNAL.wait()).await {
            Either::First(_) => {
                spedometer_led.toggle();
                if let Some(ms) = BLINK_PERIOD_SIGNAL.try_take() {
                    period_ms = ms;
                }
            }
            Either::Second(is_reverse) => {
                if is_reverse {
                    enable_led.set_high();
                } else {
                    enable_led.set_low();
                }
            }
        }
    }
}