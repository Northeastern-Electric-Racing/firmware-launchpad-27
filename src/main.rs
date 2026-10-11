#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::adc::{Adc, AdcChannel, CONTINUOUS, Exten, SampleTime};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use static_cell::StaticCell;
static DMA_BUF: StaticCell<[u16; 32]> = StaticCell::new();

mod button;
mod led;
mod potentiometer;
mod minicar;

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
    let mut reverse = embassy_stm32::exti::ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);

    //Initialize ADC input 
    let mut adc = Adc::new(p.ADC1);
    
    //create buffer
    let dma_buf = DMA_BUF.init([0u16; 32]);

    //turn pin PA3 into ADC channel
    let mut pot = p.PA3.degrade_adc();

    //build ring-buffered ADC
    let mut ring = adc.into_ring_buffered(p.DMA2_CH0, dma_buf, AdcIrqs, [(pot, SampleTime::CYCLES84)].into_iter(), embassy_stm32::adc::CONTINUOUS, Exten::DISABLED);    
    
    //call other tasks
    spawner.spawn(button::button_task(reverse, drive, brake).unwrap());
    spawner.spawn(potentiometer::potentiometer_task(ring).unwrap());
    spawner.spawn(led::led_task(led1, led2).unwrap());
    spawner.spawn(minicar::state_machine_task().unwrap());

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
