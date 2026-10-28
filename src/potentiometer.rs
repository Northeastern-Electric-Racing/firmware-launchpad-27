// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    let mut measurements = [0u16; SAMPLES];
    adc.start();

    loop {
        match adc.read(&mut measurements).await {
            Ok(_) => {
                let mut sum: u32 = 0;
                // 1. add every sample in `measurements` to `sum`
                for sample in measurements {
                    sum += sample as u32;
                }

                // 2. the average sample, as an f32
                let average: f32 = sum as f32 / SAMPLES as f32;

                // 3. the percentage, from 0.0 to 100.0
                let percent: f32 = average / MAX_TWELVE_BIT_RESOLUTION * 100.0;

                POTENTIOMETER_SIGNAL.signal(percent);
            }
            Err(_) => {
                warn!("ADC overrun");
            }
        }
    }
}
