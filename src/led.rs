
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_time::Timer;

use crate::minicar::SPEED_SIGNAL;

// Time between LED toggles for the given velocity (m/s), same rate in either direction
fn blink_period_ms(velocity: i32) -> u64 {
    match velocity.abs() {
        s if s < 25 => 2500,
        s if s < 50 => 2000,
        s if s < 75 => 1000,
        _ => 500,
    }
}

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    // Car is powered on and ready to drive
    enable_led.set_high();
    spedometer_led.set_low();

    let mut speed = 0;

    loop {
        match select(Timer::after_millis(blink_period_ms(speed)), SPEED_SIGNAL.wait()).await {
            Either::First(_) => spedometer_led.toggle(),
            // Only change the rate; the next loop iteration restarts the timer with it
            Either::Second(new_speed) => speed = new_speed,
        }
    }
}
