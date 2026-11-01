use defmt::{info, warn};
use embassy_stm32::{adc::RingBufferedAdc, peripherals::ADC1};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

const SAMPLES: usize = 128;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

pub static POTENTIOMETER_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn potentiometer_task(mut adc: RingBufferedAdc<'static, ADC1>) {
    let mut samples = [0u16; SAMPLES];
    let mut reads: u32 = 0;
    let mut overruns: u32 = 0;

    loop {
        match adc.read(&mut samples).await {
            Ok(_) => {
                let sum: u32 = samples.iter().map(|&s| s as u32).sum();
                let average = sum as f32 / SAMPLES as f32;
                let percent = average / MAX_TWELVE_BIT_RESOLUTION * 100.0;
                POTENTIOMETER_SIGNAL.signal(percent);

                reads = reads.wrapping_add(1);
                if reads.is_multiple_of(50) {
                    info!("pot = {}%", percent);
                }
            }
            Err(_) => {
                overruns = overruns.wrapping_add(1);
                if overruns.is_multiple_of(100) {
                    warn!("ADC overruns: {}", overruns);
                }
            }
        }
    }
}