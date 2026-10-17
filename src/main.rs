#![no_std]
#![no_main]

mod button;
mod led;
mod minicar;
mod potentiometer;

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, AdcChannel, RegularConversionMode, SampleTime};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use {defmt_rtt as _, panic_probe as _};

use button::button_task;
use led::led_task;
use minicar::state_machine_task;
use potentiometer::potentiometer_task;

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
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // TODO: Initialize button inputs
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
    let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    let dma_buf = cortex_m::singleton!(ADC_BUF: [u16; 32] = [0u16; 32]).unwrap();
    let adc = Adc::new_with_config(p.ADC1, Default::default());
    let pot = adc.into_ring_buffered(
        p.DMA2_CH0,
        dma_buf,
        AdcIrqs,
        [(p.PA3.degrade_adc(), SampleTime::CYCLES480)].into_iter(),
        RegularConversionMode::Continuous,
    );

    spawner.spawn(button_task(reverse_button, drive_button, brake_button).unwrap());
    spawner.spawn(potentiometer_task(pot).unwrap());
    spawner.spawn(led_task(enable_led, speedometer_led).unwrap());
    spawner.spawn(state_machine_task().unwrap());
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
