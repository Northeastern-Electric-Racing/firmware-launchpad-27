
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;

pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay).await {
        Ok(s) => s,
        Err(_) => {
            warn!("SHT30 setup failed");
            return;
        }
    };

    loop {
        match sensor.single_measurement().await {
            Ok(m) => {
                let celsius = m.temperature.celsius().value();
                TEMPERATURE_SIGNAL.signal(celsius);
            }
            Err(_) => warn!("temp read failed"),
        }
        Timer::after_millis(500).await;
    }
}