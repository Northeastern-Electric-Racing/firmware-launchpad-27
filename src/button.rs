use embassy_stm32::{exti::ExtiInput, mode::Async};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;

#[derive(Clone, Copy)]
pub enum Button {
    Drive,
    Brake,
    Reverse,
}

pub static BUTTONS: Channel<CriticalSectionRawMutex, Button, 8> = Channel::new();

#[embassy_executor::task(pool_size = 3)]
pub async fn button_task(mut input: ExtiInput<'static, Async>, which: Button) {
    loop {
        input.wait_for_rising_edge().await;
        // Try_send so a slow consumer can never stop us from watching the pin
        let _ = BUTTONS.try_send(which);
        // Debounce: ignore bounce, then wait until released
        Timer::after_millis(20).await;
        input.wait_for_low().await;
        Timer::after_millis(20).await;
    }
}