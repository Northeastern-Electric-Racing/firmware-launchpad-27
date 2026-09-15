#![no_std]
#![no_main]

use core::panic::PanicInfo;

use cortex_m::peripheral::SCB;
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Create a config for STM32F4
    let config = Config::default();

    // Intialize peripherals
    let p = embassy_stm32::init(config);

    info!("Welcome to Firmware Launchpad!");

    let mut alt = false;
    loop {
        if alt {
            info!(".");
        } else {
            info!("..");
        }

        Timer::after_millis(500).await;
    }
}

#[panic_handler]
fn hard_fault_handler(_info: &PanicInfo) -> ! {
    SCB::sys_reset(); // reset STM
}
