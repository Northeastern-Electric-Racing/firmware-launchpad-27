
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 3 ***
use embassy_stm32::{
    Peri,
    adc::{Adc, AdcChannel, AdcConfig, CONTINUOUS, Exten, Resolution, SampleTime},
    peripherals::{ADC1, DMA2_CH1, PA0},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};

// Latest position wins. Units: tenths of a percent, 0..=1000.
pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, u16> = Signal::new();
const DMA_SAMPLES: usize = 1024;

#[embassy_executor::task]
// TODO: Complete Potentiometer Task
pub async fn potentiometer_task(
    adc_peripheral: Peri<'static, ADC1>,
    pin: Peri<'static, PA0>,
    dma: Peri<'static, DMA2_CH1>,
) {
    let adc = Adc::new(
        adc_peripheral,
        AdcConfig {
            resolution: Some(Resolution::BITS12),
            ..Default::default()
        },
    );
    // Task-owned memory survives every await; no heap or static mut needed.
    let mut dma_buffer = [0u16; DMA_SAMPLES];
    let mut adc = adc.into_ring_buffered(
        dma,
        &mut dma_buffer,
        crate::AdcIrqs,
        [(pin.degrade_adc(), SampleTime::CYCLES640_5)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED,
    );
    let mut samples = [0u16; DMA_SAMPLES / 2];
    let mut last_report = Instant::now();
    loop {
        match adc.read(&mut samples).await {
            Ok(_) => {
                let sum: u32 = samples.iter().map(|&sample| u32::from(sample)).sum();
                let raw = sum / samples.len() as u32;
                // Scale before truncating: preserve fractional ADC counts at
                // gear boundaries (an average of 1023.75 is exactly 25%).
                let position_tenths =
                    (u64::from(sum) * 1000 / (samples.len() as u64 * 4095)) as u16;
                POTENTIOMETER_SIGNAL.signal(position_tenths);
                if last_report.elapsed() >= Duration::from_millis(200) {
                    defmt::info!(
                        "Potentiometer: raw={}, position={}.{}%",
                        raw,
                        position_tenths / 10,
                        position_tenths % 10
                    );
                    last_report = Instant::now();
                }
            }
            Err(_) => {
                adc.clear();
                defmt::warn!("ADC DMA overrun: discarded unread samples; retrying");
            }
        }
    }
}
