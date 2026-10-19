
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use embassy_futures::select::{Either4, select4};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker};

use crate::temp::FAULT;

pub static BLINK_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static REVERSE: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(
    mut spedometer_led: Output<'static>,
    mut enable_led: Output<'static>,
) {
    let mut ticker = Ticker::every(Duration::from_millis(2500));
    let mut fault_rx = FAULT.receiver().unwrap();
    let mut faulted = false;

    loop {
        match select4(
            BLINK_MS.wait(),
            REVERSE.wait(),
            fault_rx.changed(),
            ticker.next(),
        )
        .await
        {
            Either4::First(ms) => ticker = Ticker::every(Duration::from_millis(ms.max(20))),
            Either4::Second(in_reverse) => {
                if in_reverse {
                    enable_led.set_low()
                } else {
                    enable_led.set_high()
                }
            }
            Either4::Third(f) => {
                faulted = f;
                if faulted {
                    spedometer_led.set_low(); // off immediately
                }
            }
            Either4::Fourth(_) => {
                if !faulted {
                    spedometer_led.toggle();
                }
            }
        }
    }
}