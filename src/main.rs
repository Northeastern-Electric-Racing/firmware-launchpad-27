#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::adc::{Adc, AdcChannel, CONTINUOUS, RingBufferedAdc, SampleTime};
use embassy_stm32::exti::InterruptHandler;
use embassy_stm32::gpio::Pull::{self, Up};
use embassy_stm32::gpio::{Level, Speed};
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::peripherals::{DMA2, EXTI8, EXTI9};
use embassy_stm32::timer::low_level::OutputCompareMode::Toggle;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

mod button;
mod led;
mod minicar;
mod potentiometer;
mod temp;

bind_interrupts!(struct ExtiIrqs {
    EXTI9_5 => InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

bind_interrupts!(struct I2cIrqs {
    DMA1_STREAM0 => dma::InterruptHandler<peripherals::DMA1_CH0>;
    DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
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
    let enable_led = embassy_stm32::gpio::Output::new(p.PC8, Level::High, Speed::Low);
    let speedometer_led = embassy_stm32::gpio::Output::new(p.PC9, Level::High, Speed::Low);

    // TODO: Initialize button inputs
    let drive_button = embassy_stm32::exti::ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let brake_button = embassy_stm32::exti::ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
    let dir_button = embassy_stm32::exti::ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);

    // ADC initial
    let adc = embassy_stm32::adc::Adc::new(p.ADC1);
    let adc_pin = p.PA3.degrade_adc();
    let mut dma_buf = cortex_m::singleton!(: [u16; 32] = [0 ; 32]).unwrap();

    //configure into_ring_buffered()
    let adc = adc.into_ring_buffered(
        p.DMA2_CH0,
        dma_buf,
        AdcIrqs,
        [(adc_pin, SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        embassy_stm32::adc::Exten::DISABLED,
    );

    let i2c = I2c::new(
        p.I2C1,
        p.PB6,      // SCL
        p.PB7,      // SDA
        p.DMA1_CH7, // TX DMA
        p.DMA1_CH0, // RX DMA
        I2cIrqs,
        i2c::Config::default(),
    );

    spawner.spawn(button::button_task(dir_button, drive_button, brake_button).unwrap());
    spawner.spawn(led::led_task(enable_led, speedometer_led).unwrap());
    spawner.spawn(potentiometer::potentiometer_task(adc).unwrap());
    spawner.spawn(minicar::state_machine_task().unwrap());
    spawner.spawn(temp::temp_task(i2c).unwrap());
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
