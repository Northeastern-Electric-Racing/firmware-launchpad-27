
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***

use defmt::{info};
use embassy_stm32::{Peri, adc::{Adc, AdcChannel, CONTINUOUS, Exten, SampleTime}, peripherals::{self}};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

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
 
    let mut ring = adc.into_ring_buffered(
        dma,
        &mut dma_buf,
        AdcIrqs,
        [(pedal, SampleTime::CYCLES144)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED,
    );
 
    let mut samples = [0u16; 128]; // must be exactly half of dma_buf
 
    loop {
        match ring.read(&mut samples).await {
            Ok(n) if n > 0 => {
                let sum: u32 = samples[..n].iter().map(|&s| s as u32).sum();
                let avg = sum / n as u32;
                let pct = avg * 100 / 4095; 
                PEDAL_PCT.signal(pct);
                info!("pedal: {}%", pct);
            }
            Ok(_) => {}
            Err(_) => info!("ADC ring buffer overrun"),
        }
    Timer::after_millis(300).await;
    }
}
