
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

#[derive(Clone, Copy)]
pub struct LedState { pub enabled: bool, pub velocity: i32 }

pub static LED_SIGNAL: Signal<CriticalSectionRawMutex, LedState> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) { 

 // TODO: LED Task
let mut state = LedState { enabled: false, velocity: 0 };
loop {
    if state.enabled { enable_led.set_high(); } else { enable_led.set_low(); }

    if !state.enabled || state.velocity == 0 {
        spedometer_led.set_low();
        state = LED_SIGNAL.wait().await;
    } else {
        spedometer_led.toggle();
        let delay = (1000 / state.velocity.unsigned_abs().max(1)).clamp(50, 500) as u64;
        if let Either::Second(s) = select(Timer::after_millis(delay), LED_SIGNAL.wait()).await {
            state = s;
        }
    }
}
}
  
