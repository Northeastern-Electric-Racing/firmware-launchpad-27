
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;
// Preserve each temperature/failure report: do not overwrite an unread hot
// reading with a subsequent cool one. None means acquisition failed.
pub static TEMPERATURE_EVENTS: Channel<CriticalSectionRawMutex, Option<f32>, 8> = Channel::new();

#[embassy_executor::task]
pub async fn temp_task(mut i2c: I2c<'static, Async, Master>) {
    // TODO: Complete Temperature Task
    loop {
        // Allow power-up, then use the constructor's software reset. The
        // sensor nRESET must remain deasserted by its pull-up, not by PB8/SCL.
        Timer::after_millis(20).await;
        match Sht3x::new(&mut i2c, SHT30_ADDRESS, Delay).await {
            Ok(mut sensor) => loop {
                match sensor.single_measurement().await {
                    Ok(measurement) => {
                        // Driver validates CRC and applies datasheet section
                        // 4.13: T(C) = -45 + 175 * raw / 65535. Do not convert twice.
                        let celsius = measurement.temperature.value();
                        TEMPERATURE_EVENTS.send(Some(celsius)).await;
                        defmt::info!("Temperature: {} C", celsius);
                    }
                    Err(error) => {
                        TEMPERATURE_EVENTS.send(None).await;
                        defmt::warn!(
                            "SHT30 measurement failed: {:?}",
                            defmt::Debug2Format(&error)
                        );
                        break;
                    }
                }
                Timer::after_millis(250).await;
            },
            Err(error) => {
                TEMPERATURE_EVENTS.send(None).await;
                defmt::warn!(
                    "SHT30 initialization failed: {:?}",
                    defmt::Debug2Format(&error)
                );
            }
        }
        Timer::after_millis(250).await;
    }
}
