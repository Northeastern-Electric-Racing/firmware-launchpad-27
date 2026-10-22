// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::{info, warn};
use embassy_futures::yield_now;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

const SAMPLES: usize = 128;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    let mut readings = [0u16; SAMPLES];
    let mut oks: u32 = 0;
    let mut overruns: u32 = 0;

    loop {
        match adc.read(&mut readings).await {
            Ok(_) => {
                oks += 1;
                let total: u32 = readings.iter().map(|&x| x as u32).sum();
                let avg = total as f32 / SAMPLES as f32;
                let percent = avg / MAX_TWELVE_BIT_RESOLUTION * 100.0;
                POTENTIOMETER_SIGNAL.signal(percent);
                if oks % 100 == 0 {
                    info!("pot ok #{}: {}%", oks, percent);
                }
            }
            Err(_) => {
                overruns += 1;
                if overruns % 100 == 0 {
                    warn!("ADC overruns: {} (ok reads: {})", overruns, oks);
                }
            }
        }
        yield_now().await;
    }
}