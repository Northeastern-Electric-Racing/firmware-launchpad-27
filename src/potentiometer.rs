
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::{warn};
use embassy_stm32::adc::{Adc, AnyAdcChannel, CONTINUOUS, Exten, SampleTime};
use embassy_stm32::peripherals::DMA2_CH0;
use embassy_stm32::{Peri};
use embassy_stm32::{peripherals::ADC1};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

use crate::AdcIrqs;

pub const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(
    adc: Adc<'static, ADC1>,
    adc_pin: AnyAdcChannel<'static, ADC1>,
    dma: Peri<'static, DMA2_CH0>,
) {
    let mut dma_adc_buf = [0u16; SAMPLES*2];

    let mut adc = adc.into_ring_buffered(
        dma, 
        &mut dma_adc_buf,
        AdcIrqs,
        [(adc_pin, SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED
    );

    let mut measurements = [0u16; SAMPLES*2];

    loop {
        adc.clear();
        match adc.read(&mut measurements).await {
            Ok(_)=> {
                let mut average: f32 = 0.0;
                for i in measurements.iter().take(SAMPLES).skip(1){
                    average += *i as f32 / MAX_TWELVE_BIT_RESOLUTION;
                }
                average /= SAMPLES as f32;
                POTENTIOMETER_SIGNAL.signal(average);
            }
            Err(e) => {
                warn!("Error {} occured", e);
            }
        }
    }
}
