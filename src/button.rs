
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

pub use crate::input::ButtonType;
use crate::input::{ButtonPress, Debouncer};
use embassy_futures::select::{Either, select, select3};
use embassy_stm32::{exti::ExtiInput, mode::Async};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Instant, Timer};

// Every press counts. A Signal would overwrite an unread press with a new one.
pub static BUTTON_EVENTS: Channel<CriticalSectionRawMutex, ButtonPress, 16> = Channel::new();

#[embassy_executor::task]
pub async fn button_task(
    dir_button: ExtiInput<'static, Async>,
    accel_button: ExtiInput<'static, Async>,
    brake_button: ExtiInput<'static, Async>,
) {
    select3(
        report_presses(dir_button, ButtonType::Direction),
        report_presses(accel_button, ButtonType::Accelerate),
        report_presses(brake_button, ButtonType::Brake),
    )
    .await;
}
async fn report_presses(mut button: ExtiInput<'_, Async>, event: ButtonType) -> ! {
    let mut debouncer = Debouncer::new(button.is_low(), Instant::now().as_millis());
    let mut edge_seen = false;
    loop {
        // TODO: Complete Button Task
        if let Some(started_ms) =
            debouncer.update(button.is_low(), Instant::now().as_millis(), edge_seen)
        {
            BUTTON_EVENTS
                .send(ButtonPress {
                    kind: event,
                    started_ms,
                })
                .await;
            // Recheck the pin after possible queue backpressure before sleeping.
            edge_seen = false;
            continue;
        }
        edge_seen = match debouncer.deadline_ms() {
            Some(deadline) => matches!(
                select(
                    button.wait_for_any_edge(),
                    Timer::at(Instant::from_millis(deadline))
                )
                .await,
                Either::First(())
            ),
            None => {
                // Level waits avoid missing an edge between checking and arming.
                if debouncer.stable_pressed() {
                    button.wait_for_high().await;
                } else {
                    button.wait_for_low().await;
                }
                true
            }
        };
    }
}
