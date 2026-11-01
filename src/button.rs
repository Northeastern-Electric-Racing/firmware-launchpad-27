use embassy_futures::select::{Either3, select3};
use embassy_stm32::{exti::ExtiInput, mode::Async};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use crate::button::ButtonType::{Accelerate, Brake, Direction};

pub enum ButtonType {
    Direction,
    Accelerate,
    Brake,
}

pub static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ButtonType> = Signal::new();

#[embassy_executor::task]
pub async fn button_task(
    mut dir_button: ExtiInput<'static, Async>,
    mut accel_button: ExtiInput<'static, Async>,
    mut brake_button: ExtiInput<'static, Async>,
) {
    loop {
        let dir_event = dir_button.wait_for_falling_edge();
        let accel_event = accel_button.wait_for_falling_edge();
        let brake_event = brake_button.wait_for_falling_edge();

        let pressed = match select3(dir_event, accel_event, brake_event).await {
            Either3::First(_) => Direction,
            Either3::Second(_) => Accelerate,
            Either3::Third(_) => Brake,
        };

        Timer::after_millis(20).await; // debounce: let the switch contacts settle before confirming the press

        let still_pressed = match pressed {
            Direction => dir_button.is_low(),
            Accelerate => accel_button.is_low(),
            Brake => brake_button.is_low(),
        };

        if still_pressed {
            BUTTON_SIGNAL.signal(pressed);
        }
    }
}
