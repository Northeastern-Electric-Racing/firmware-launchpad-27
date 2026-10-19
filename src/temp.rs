use defmt::{info, warn};
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, Temperature};

const SHT30_ADDRESS: u8 = 0x44;
const FAULT_TEMP_C: f32 = 35.0;

/// true = faulted. Watch lets both car_task and speedometer_task observe it.
pub static FAULT: Watch<CriticalSectionRawMutex, bool, 2> = Watch::new();

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    let mut sensor = match Sht3x::new(i2c, SHT30_ADDRESS, Delay).await {
        Ok(s) => s,
        Err(_) => {
            warn!("SHT30 init failed");
            return;
        }
    };

    let tx = FAULT.sender();
    let mut faulted = false;
    tx.send(faulted);

    loop {
        match sensor.single_measurement().await {
            Ok(m) => {
                let temp_c = m.temperature.celsius().value();

                if !faulted && temp_c > FAULT_TEMP_C {
                    faulted = true;
                    tx.send(true);
                    warn!("FAULT: over temperature");
                } else if faulted && temp_c < FAULT_TEMP_C {
                    faulted = false;
                    tx.send(false);
                    info!("fault cleared");
                }
            }
            Err(_) => warn!("SHT30 read failed"),
        }
        Timer::after_millis(500).await;
    }
}