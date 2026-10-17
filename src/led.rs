use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

pub static SPEEDOMETER_SIGNAL: Signal<CriticalSectionRawMutex, u64> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut enable_led: Output<'static>, mut spedometer_led: Output<'static>) {
    let mut toggle_period_ms = SPEEDOMETER_SIGNAL.wait().await;

    loop {
        spedometer_led.toggle();

        if let Either::Second(new_period_ms) =
            select(Timer::after_millis(toggle_period_ms), SPEEDOMETER_SIGNAL.wait()).await
        {
            toggle_period_ms = new_period_ms;
        }
    }
}
