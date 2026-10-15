#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info};
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, AdcChannel};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use {defmt_rtt as _, panic_probe as _};

mod potentiometer;
mod minicar;
mod button;
mod led;
//mod temp;

use potentiometer::*;
use minicar::*;
use button::*;
use led::*;
//use temp::*;

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

    // Initialize LED outputs
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let spedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // Initialize button inputs
    let drive_button  = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
    let direction_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);

    //Init ADC
    let adc = Adc::new(p.ADC1);


    // Init IWDG
    let mut _alt = false;

    //spawn tasks
    spawner.spawn(button_task(drive_button, direction_button, brake_button).unwrap());
    spawner.spawn(potentiometer_task(adc, p.PA3.degrade_adc(), p.DMA2_CH0).unwrap());
    spawner.spawn(led_task(enable_led, spedometer_led).unwrap());
    spawner.spawn(state_machine_task().unwrap());

    // Loop for petting IWDG
    loop {
        
    }
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
