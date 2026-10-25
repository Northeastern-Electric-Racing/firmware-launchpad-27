use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;

pub static TEMPERATURE_SIGNAL: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay) {
        Ok(sensor) => sensor,
        Err(_) => {
            warn!("SHT30 initialization failed");
            return;
        }
    };

    loop {
        match sensor.single_measurement() {
            Ok(measurement) => {
                let temperature = measurement.temperature.celsius().value();

                TEMPERATURE_SIGNAL.signal(temperature);
            }

            Err(_) => {
                warn!("SHT30 measurement failed");
            }
        }

        Timer::after_millis(1000).await;
    }
}
