
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
    let mut buf = [0u16; SAMPLES];
    loop {
        match adc.read(&mut buf).await {
            Ok(_) => {
                let sum: u32 = buf.iter().map(|&s| s as u32).sum();
                let avg = sum as f32 / SAMPLES as f32;
                POTENTIOMETER_SIGNAL.signal(avg / MAX_TWELVE_BIT_RESOLUTION);
            }
        Err(_) => warn!("ADC read failed"),
    }
}
}
