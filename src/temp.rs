use defmt::{info, warn};
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;
const SAMPLE_PERIOD_MS: u64 = 1000;

pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay) {
        Ok(sensor) => sensor,
        Err(_) => {
            warn!("SHT30 not responding at 0x44");
            return;
        }
    };

    loop {
        match sensor.single_measurement().await {
            Ok(measurement) => {
                let celsius = measurement.temperature.celsius().value();
                info!("temperature = {} C", celsius);
                TEMPERATURE_SIGNAL.signal(celsius);
            }
            Err(_) => warn!("SHT30 read failed"),
        }

        Timer::after_millis(SAMPLE_PERIOD_MS).await;
    }
}