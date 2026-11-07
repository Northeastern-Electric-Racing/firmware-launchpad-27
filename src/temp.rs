
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Timer};
use embedded_sht3x::Sht3x;

const SHT30_ADDRESS: u8 = 0x44;
const SAMPLE_PERIOD_MS: u64 = 1000;

// Latest temperature reading in °C
pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay).await {
        Ok(sensor) => sensor,
        Err(_) => {
            warn!("Temperature: failed to initialize SHT30");
            return;
        }
    };

    loop {
        match sensor.single_measurement().await {
            Ok(measurement) => TEMPERATURE_SIGNAL.signal(measurement.temperature.0),
            Err(_) => warn!("Temperature: failed to read SHT30"),
        }

        Timer::after_millis(SAMPLE_PERIOD_MS).await;
    }
}
