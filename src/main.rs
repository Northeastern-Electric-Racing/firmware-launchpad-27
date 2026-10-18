#![no_std]
#![no_main]

use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Ticker, Timer};
use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info};
use embassy_executor::Spawner;
use embassy_futures::select::Either::{First, Second};
use embassy_futures::select::{select};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_stm32::gpio::{Level, Output, Speed, Pull};
use crate::potentiometer::potentiometer_task;
use crate::led::led_task;
use crate::button::{Button, button_task};
use crate::minicar::car_task;

use {defmt_rtt as _, panic_probe as _};
use defmt::unwrap;

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

#[embassy_executor::task]
async fn buttons_task(
    mut brake_button: ExtiInput<'static, Async>,
    mut drive_button: ExtiInput<'static, Async>,
    mut speedom_led: Output<'static>,
    mut enable_led: Output<'static>,
) {

    let mut tick = Ticker::every(Duration::from_millis(10));

    loop {
        match select3(brake_button.wait_for_any_edge(), drive_button.wait_for_any_edge(), tick.next()).await{
            Either3::First(_) | Either3::Second(_) => Timer::after_millis(5).await,
            Either3::Third(_) => {}
        }
        speedom_led.set_level(brake_button.is_low().into());
        enable_led.set_level(drive_button.is_low().into());
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Config::default());
 
    info!("Welcome to Firmware Launchpad!");
 
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let speedom_led = Output::new(p.PC9, Level::Low, Speed::Low);
 
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
    let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
 
    //Phase 3
    // spawner.spawn(button_task(drive_button, Button::Drive).unwrap());
    // spawner.spawn(button_task(brake_button, Button::Brake).unwrap());
    // spawner.spawn(button_task(reverse_button, Button::Reverse).unwrap());
    // spawner.spawn(unwrap!(potentiometer_task(p.ADC1, p.PA3, p.DMA2_CH0)));
    // spawner.spawn(unwrap!(led_task(speedom_led, enable_led)));
    // spawner.spawn(unwrap!(car_task()));
    
    //Phase 1/2
    spawner.spawn(unwrap!(buttons_task(brake_button, drive_button, speedom_led, enable_led)));
    spawner.spawn(unwrap!(potentiometer_task(p.ADC1, p.PA3, p.DMA2_CH0)));
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
