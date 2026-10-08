#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::exti::InterruptHandler;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct ExtiIrqs {
    EXTI9_5 => InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

bind_interrupts!(struct I2cIrqs {
    DMA1_STREAM2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
    DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
    I2C2_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C2>;
});

bind_interrupts!(struct AdcIrqs {
    DMA2_STREAM0 => dma::InterruptHandler<peripherals::DMA2_CH0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Create a config for STM32F4
    let config = Config::default();

    // intialize peripherals
    let p = embassy_stm32::init(config);

    // *** FIRMWARE LAUNCHPAD: START HERE! ***
    info!("Welcome to Firmware Launchpad!");

    // TODO: Initialize LED outputs
    let mut led1 = Output::new(p.PC8, Level::Low, Speed::Low);
    let mut led2 = Output::new(p.PC9, Level::Low, Speed::Low);

    // TODO: Initialize button inputs
    let mut drive = embassy_stm32::exti::ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let mut brake = embassy_stm32::exti::ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    loop {
        match select(drive.wait_for_falling_edge(), brake.wait_for_falling_edge()).await {
            Either::First(_) => {
                led1.toggle();
            } 
            Either::Second(_) => {
                led2.toggle();
            }
        }
    }

    // Init IWDG
    let mut alt = false;

    // Loop for petting IWDG
    loop {
        if !alt {
            info!(".");
        } else {
            info!("..");
        }
        alt = !alt;
        Timer::after_millis(100).await;
    }
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
