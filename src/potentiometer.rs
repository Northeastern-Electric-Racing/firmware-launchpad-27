
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;
pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    // TODO: Complete Potentiometer Task
        let mut measurements = [0u16; SAMPLES];

    loop {
        adc.clear();
        match adc.read(&mut measurements).await {
            Ok(_) => {
                let mut sum: u32 = 0;
                for measurement in measurements {
                    sum += measurement as u32;
                }

                let avg = sum / SAMPLES as u32;

                let percentage = avg as f32 / MAX_TWELVE_BIT_RESOLUTION * 100.0;

                POTENTIOMETER_SIGNAL.signal(percentage)
            }

            Err(e) => {
                warn!("ADC DMA overrun: {}", e);
            }
        }

        Timer::after_millis(100).await;
    }
}
