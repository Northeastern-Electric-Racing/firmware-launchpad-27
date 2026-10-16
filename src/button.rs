
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either3, select3};
use embassy_stm32::{exti::ExtiInput, mode::Async};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub enum Button {
    Drive,
    Brake,
    Reverse,
}

pub static BUTTONS: Channel<CriticalSectionRawMutex, Button, 4> = Channel::new();

#[embassy_executor::task]
pub async fn button_task(
    mut drive: ExtiInput<'static, Async>,
    mut brake: ExtiInput<'static, Async>,
    mut reverse: ExtiInput<'static, Async>
) {
    loop {
        let button = match select3(
            drive.wait_for_rising_edge(),
            brake.wait_for_rising_edge(),
            reverse.wait_for_rising_edge(),
        )
        .await
        {
            Either3::First(_) => Button::Drive,
            Either3::Second(_) => Button::Brake,
            Either3::Third(_) => Button::Reverse,
        };

        BUTTONS.send(button).await;
    }
}
