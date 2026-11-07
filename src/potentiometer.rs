// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;
const SAMPLE_PERIOD_MS: u64 = 50;

// Latest potentiometer position as a percentage of rotation (0.0 - 100.0)
pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    let mut measurements = [0u16; SAMPLES];

    loop {
        Timer::after_millis(SAMPLE_PERIOD_MS).await;

        // Grab only the most recent samples; older data in the ring buffer is discarded
        let count = adc.read_latest(&mut measurements);
        if count == 0 {
            warn!("Potentiometer: no ADC samples available");
            continue;
        }

        // Average the samples to smooth out noise on the reading
        let sum: u32 = measurements[..count].iter().map(|&s| s as u32).sum();
        let average = sum as f32 / count as f32;

        let percentage = average / MAX_TWELVE_BIT_RESOLUTION * 100.0;
        POTENTIOMETER_SIGNAL.signal(percentage);
    }
}
