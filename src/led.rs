
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either3, select3};
use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Instant, Timer};

use crate::minicar::{FAULT_SIGNAL, SPEED_SIGNAL};

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

    let mut period = Duration::from_millis(blink_period_ms(0));
    let mut faulted = false;
    // Kept across iterations so unrelated signals don't restart the blink timer
    let mut next_toggle = Instant::now() + period;

    loop {
        // Faulted: speedometer LED off, no blinking until the fault clears
        if faulted {
            spedometer_led.set_low();
            faulted = FAULT_SIGNAL.wait().await;
            if !faulted {
                next_toggle = Instant::now() + period;
            }
            continue;
        }

        match select3(
            Timer::at(next_toggle),
            SPEED_SIGNAL.wait(),
            FAULT_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(_) => {
                spedometer_led.toggle();
                next_toggle += period;
            }
            // Only restart the timer when the speed moves into a different blink rate
            Either3::Second(new_speed) => {
                let new_period = Duration::from_millis(blink_period_ms(new_speed));
                if new_period != period {
                    period = new_period;
                    next_toggle = Instant::now() + period;
                }
            }
            // A repeated fault value leaves the timer untouched
            Either3::Third(new_faulted) => faulted = new_faulted,
        }
    }
}
