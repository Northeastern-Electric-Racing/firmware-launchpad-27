
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::{info};
use embassy_stm32::{Peri, adc::{Adc, AdcChannel, CONTINUOUS, Exten, SampleTime}, peripherals::{self}};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::AdcIrqs;

pub static PEDAL_PCT: Signal<CriticalSectionRawMutex, u32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(
    adc: Peri<'static, peripherals::ADC1>,
    pin: Peri<'static, peripherals::PA3>,
    dma: Peri<'static, peripherals::DMA2_CH0>,
) {
    let adc = Adc::new(adc);
    let pedal = pin.degrade_adc();
 
    let mut dma_buf = [0u16; 256];
    let mut samples = [0u16; 128]; // must be exactly half of dma_buf
 
    let mut ring = adc.into_ring_buffered(
        dma,
        &mut dma_buf,
        AdcIrqs,
        [(pedal, SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED,
    );

    let mut count = 0u32;

    loop {
        ring.clear();
        match ring.read(&mut samples).await {
            Ok(_remaining) => {
                let sum: u32 = samples.iter().map(|&s| s as u32).sum();
                let pct = sum / samples.len() as u32 * 100 / 4095;
                PEDAL_PCT.signal(pct);

                count += 1;
                if count % 20 == 0 {
                    info!("pedal: {}%", pct);
                }
            }
            Err(_) => {
                defmt::warn!("ADC ring overrun");
            }
        }
    }
}
