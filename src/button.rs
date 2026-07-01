
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_stm32::{exti::ExtiInput, mode::Async};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

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

        // TODO: Complete Button Task
    }
}
