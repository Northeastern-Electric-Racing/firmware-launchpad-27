
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::warn;
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    // TODO: Complete Potentiometer Task
}
