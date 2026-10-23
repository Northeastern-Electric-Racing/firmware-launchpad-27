#![no_std]
#![no_main]

mod button;
mod led;
mod minicar;
mod potentiometer;
mod temp;

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, AdcChannel, AdcConfig, Exten, SampleTime};
use embassy_stm32::rcc::mux::Adcsel;
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use {defmt_rtt as _, panic_probe as _};

use button::button_task;
use led::led_task;
use minicar::state_machine_task;
use potentiometer::potentiometer_task;
use temp::temp_task;

bind_interrupts!(struct ExtiIrqs {
    EXTI9_5 => InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

bind_interrupts!(struct I2cIrqs {
    DMA1_CHANNEL1 => dma::InterruptHandler<peripherals::DMA1_CH1>;
    DMA1_CHANNEL2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

bind_interrupts!(struct AdcIrqs {
    DMA2_CHANNEL1 => dma::InterruptHandler<peripherals::DMA2_CH1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Create a config for STM32G4
    let mut config = Config::default();
    config.rcc.mux.adc12sel = Adcsel::SYS;

    // intialize peripherals
    let p = embassy_stm32::init(config);

    // *** FIRMWARE LAUNCHPAD: START HERE! ***
    info!("Welcome to Firmware Launchpad!");

    // TODO: Initialize LED outputs
    let enable_led = Output::new(p.PB0, Level::Low, Speed::Low);
    let speedometer_led = Output::new(p.PA5, Level::Low, Speed::Low);

    // TODO: Initialize button inputs
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
    let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    let dma_buf = cortex_m::singleton!(ADC_BUF: [u16; 256] = [0u16; 256]).unwrap();
    let adc = Adc::new(p.ADC1, AdcConfig::default());
    let pot = adc.into_ring_buffered(
        p.DMA2_CH1,
        dma_buf,
        AdcIrqs,
        [(p.PA0.degrade_adc(), SampleTime::CYCLES640_5)].into_iter(),
        embassy_stm32::adc::CONTINUOUS,
        Exten::DISABLED,
    );

    let i2c = I2c::new(
        p.I2C1,
        p.PB8,
        p.PB9,
        p.DMA1_CH1,
        p.DMA1_CH2,
        I2cIrqs,
        Default::default(),
    );

    spawner.spawn(button_task(reverse_button, drive_button, brake_button).unwrap());
    spawner.spawn(potentiometer_task(pot).unwrap());
    spawner.spawn(led_task(enable_led, speedometer_led).unwrap());
    spawner.spawn(state_machine_task().unwrap());
    spawner.spawn(temp_task(i2c).unwrap());
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
