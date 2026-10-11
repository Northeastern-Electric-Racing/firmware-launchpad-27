
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    // TODO: Complete Potentiometer Task
    let mut readings = [0u16; SAMPLES];

    pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

    let mut readings = [0u16; SAMPLES];
    loop {
    match adc.read(&mut readings).await {
        Ok(n) => {
            let total: u32 = readings[..n].iter().map(|&x| x as u32).sum();
            let avg = total as f32 / n as f32;
            let percent = avg / MAX_TWELVE_BIT_RESOLUTION * 100.0;
            POTENTIOMETER_SIGNAL.signal(percent);
            }
        Err(_) => warn!("ADC overrun"),
        }
    }
}
