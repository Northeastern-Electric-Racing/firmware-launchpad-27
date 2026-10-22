
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embedded_sht3x::Sht3x;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Timer};


const SHT30_ADDRESS: u8 = 0x44;
pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    // TODO: Complete Temperature Task
    // inside temp_task, replacing the TODO:
    let Ok(mut sensor) = Sht3x::new(i2c, SHT30_ADDRESS, Delay) 
    else {
        warn!("SHT30 init failed");
        return;
    };
    loop {
        match sensor.single_measurement() {
            Ok(reading) => TEMPERATURE_SIGNAL.signal(reading.temperature.0),
            Err(_) => warn!("SHT30 read failed"),
        }
        Timer::after_millis(1000).await;
    }
}

