#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_futures::select::Either::{First, Second};
use embassy_stm32::adc::{Adc, AdcChannel, RegularConversionMode, SampleTime};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, Peri, bind_interrupts, dma, interrupt,
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
    // let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    // let speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // TODO: Initialize button inputs
    // let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    // let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    //Task-1 commented now
    // buttons(enable_led, speedometer_led, drive_button, brake_button).await;

    //Task-2 
    potentiometer(p.ADC1, p.DMA2_CH0, p.PA3).await;
}

async fn buttons(
    mut enable_led: Output<'_>,
    mut speedometer_led: Output<'_>,
    mut drive_button: ExtiInput<'_, Async>,
    mut brake_button: ExtiInput<'_, Async>,
) {
    loop {
        let brake = brake_button.wait_for_any_edge();
        let drive = drive_button.wait_for_any_edge();

        match select(brake, drive).await {
            First(_) => {
                Timer::after_millis(20).await;
                info!("Brake pressed");
                speedometer_led.set_level(
                    if brake_button.is_low() {
                        {Level::High}
                    } else{
                        {Level::Low}
                    }
                );
            }
            Second(_) => {
                Timer::after_millis(20).await;
                info!("Drive pressed");
                enable_led.set_level(
                    if drive_button.is_low() { 
                        {Level::High} 
                    } else{ 
                        {Level::Low}
                    }
                );
            }
        }
    }
}

async fn potentiometer(
    adc1: Peri<'_, peripherals::ADC1>,
    dma: Peri<'_, peripherals::DMA2_CH0>,
    pot_pin: Peri<'_, peripherals::PA3>,
) {
    const DMA_BUF_LEN: usize = 64;
    let mut dma_buf = [0u16; DMA_BUF_LEN];
    let mut samples = [0u16; DMA_BUF_LEN / 2];

    let adc = Adc::new_with_config(adc1, Default::default());
    let mut pot = adc.into_ring_buffered(
        dma,
        &mut dma_buf,
        AdcIrqs,
        [(pot_pin.degrade_adc(), SampleTime::CYCLES480)].into_iter(),
        RegularConversionMode::Continuous,
    );
    pot.start();

    let mut count: u32 = 0;
    loop {
        match pot.read(&mut samples).await {
            Ok(_) => {
                let sum: u32 = samples.iter().map(|&s| s as u32).sum();
                let avg = sum / samples.len() as u32;
                let percent = avg * 100 / 4095;

                count += 1;
                if count % 100 == 0 {
                    info!("raw = {}, pot = {}%", avg, percent);
                }
            }
            Err(_) => {
                warn!("ADC overrun, restarting");
                pot.start();
            }
        }
    }
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
