// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer};

// None = fault: force off and wait. Some(ms) = normal blink timing.
// Queue transitions: a recovery command must not overwrite an unread fault-off.
pub static LED_INTERVAL: Channel<CriticalSectionRawMutex, Option<u64>, 8> = Channel::new();

#[embassy_executor::task]
    // TODO: LED Task
pub async fn led_task(mut enable_led: Output<'static>, mut speedometer_led: Output<'static>) {
    enable_led.set_high(); // Running indicator; fault behavior applies to Speedometer.
    let mut interval = LED_INTERVAL.receive().await;
    loop {
        let Some(milliseconds) = interval else {
            speedometer_led.set_low();
            interval = LED_INTERVAL.receive().await;
            continue;
        };
        let deadline = Instant::now() + Duration::from_millis(milliseconds);
        // A fault update wins over a timer that is ready at the same poll.
        match select(LED_INTERVAL.receive(), Timer::at(deadline)).await {
            Either::First(new_interval) => interval = new_interval,
            Either::Second(()) => speedometer_led.toggle(),
        }
    }
}
