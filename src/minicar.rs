// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***
use crate::{
    button::{BUTTON_EVENTS, ButtonType},
    car::MiniCar,
    led::LED_INTERVAL,
    potentiometer::POTENTIOMETER_SIGNAL,
    temp::TEMPERATURE_EVENTS,
};
// TODO: Complete the Minicar implementation! (moved to car.rs)
use embassy_futures::select::{Either3, select3};
use embassy_time::Instant;

#[embassy_executor::task]
pub async fn state_machine_task() {
    // Sole owner/writer. Startup is faulted until a valid temperature below 35 C.
    let mut car = MiniCar::new();
    let mut interval = car.led_interval_ms();
    LED_INTERVAL.send(interval).await;
    let mut ready_since_ms = Instant::now().as_millis();
    defmt::info!(
        "Car waiting for valid cool temperature: {:?}",
        defmt::Debug2Format(&car)
    );
    loop {
        // TODO: Complete State Machine Task
        let previous = car;
        // Temperature has priority when several inputs are already ready.
        // Do not block temperature handling while waiting for initial ADC data.
        match select3(
            TEMPERATURE_EVENTS.receive(),
            BUTTON_EVENTS.receive(),
            POTENTIOMETER_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(temperature) => car.update_temperature(temperature),
            Either3::Second(press) => {
                // Also reject fault-time presses still in debounce/queue send
                // when recovery clears the already-queued button events.
                if !press.is_fresh(ready_since_ms) {
                    continue;
                }
                if let Some(position) = POTENTIOMETER_SIGNAL.try_take() {
                    car.set_position(position);
                }
                match press.kind {
                    ButtonType::Accelerate => car.accelerate(),
                    ButtonType::Brake => car.brake(),
                    ButtonType::Direction => car.reverse(),
                }
            }
            Either3::Third(position) => car.set_position(position),
        }
        if car.faulted() != previous.faulted() {
            // Never replay queued startup/fault-time presses after recovery.
            BUTTON_EVENTS.clear();
            ready_since_ms = Instant::now().as_millis();
            defmt::info!(
                "Temperature fault={}; queued buttons discarded",
                car.faulted()
            );
        }
        let new_interval = car.led_interval_ms();
        if new_interval != interval {
            interval = new_interval;
            LED_INTERVAL.send(interval).await;
        }
        if car != previous {
            defmt::info!(
                "Car: {:?}; Speedometer interval(ms)={:?}",
                defmt::Debug2Format(&car),
                interval
            );
        }
    }
}
