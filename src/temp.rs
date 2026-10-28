// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::{info, warn};
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Repeatability, Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;

pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    // TODO: Complete Temperature Task
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay) {
        Ok(sensor) => sensor,
        Err(_) => {
            warn!("Sht30 not responding");
            return;
        }
    };
    sensor.repeatability = Repeatability::High;

    loop {
        match sensor.single_measurement() {
            Ok(measurement) => {
                let celcius: f32 = measurement.temperature.value();
                TEMPERATURE_SIGNAL.signal(celcius);
                info!("temperature: {} C", celcius);
            }
            Err(_) => warn!("SHT30 read failed"),
        }
        Timer::after_millis(500).await
    }
}
