// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Repeatability::High, Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;

pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    // TODO: Complete Temperature Task
    let mut temp_sensor;
    match Sht3x::new(i2c, SHT30_ADDRESS, Delay) {
        Ok(sensor) => {
            temp_sensor = sensor;
            temp_sensor.repeatability = High;
        }
        Err(_) => {
            warn!("Sht30 initilization error occured");
            return;
        }
    }

    loop {
        let measurement = match temp_sensor.single_measurement() {
            Ok(value) => value,
            Err(_) => {
                warn!("Sht30 measurement error occured");
                break;
            }
        };
        TEMPERATURE_SIGNAL.signal(measurement.temperature.celsius().value());
        Timer::after_millis(500).await;
    }
}
