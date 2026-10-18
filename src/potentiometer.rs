// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    // TODO: Complete Potentiometer Task
    let mut measurements = [0u16; SAMPLES];
    let mut last_percent: u32 = 1000;

    adc.start();

    loop {
        match adc.read(&mut measurements).await {
            Ok(_) => {
                let mut total: u32 = 0;
                for value in measurements {
                    total += value as u32;
                }
                let average = total as f32 / SAMPLES as f32;

                let percent = average / MAX_TWELVE_BIT_RESOLUTION * 100.0;

                if percent as u32 != last_percent {
                    last_percent = percent as u32;
                    POTENTIOMETER_SIGNAL.signal(percent);
                }
            }
            Err(_) => {
                warn!("adc overrun");
                adc.start();
            }
        }
    }
}
