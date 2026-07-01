
// *** IGNORE THIS FILE UNTIL FIRMWARE LAUNCHPAD: PHASE 4 ***

use defmt::warn;
use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embedded_sht3x::Sht3x;

const SHT30_ADDRESS: u8 = 0x44;

#[embassy_executor::task]
pub async fn temp_task(i2c: I2c<'static, Async, Master>) {
    // TODO: Complete Temperature Task
}
