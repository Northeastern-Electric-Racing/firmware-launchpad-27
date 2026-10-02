
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_stm32::gpio::Output;

#[embassy_executor::task]

pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    // TODO: LED Task
}
