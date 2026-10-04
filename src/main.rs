#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::Either::{First, Second};
use embassy_futures::select::select;
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_stm32::adc::{Adc, Exten, AdcChannel, CONTINUOUS, SampleTime};
use embassy_stm32::gpio::{Level, Output, Speed, Pull};
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
    let mut enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let mut speedom_led = Output::new(p.PC9, Level::Low, Speed::Low);
    
    let mut break_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
    let mut drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);

    let pot_adc = Adc::new(p.ADC1);
    let pot_pedal = p.PA3.degrade_adc();
    let mut dma_buf = [0u16; 256];
    let mut samples = [0u16; 16];

    let mut ring = pot_adc.into_ring_buffered(
        p.DMA2_CH0,
        &mut dma_buf,
        AdcIrqs,
        [(pot_pedal, SampleTime::CYCLES144)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED
    );

    let buttons = async {
        loop {
            let brake_fut = break_button.wait_for_rising_edge();
            let drive_fut = drive_button.wait_for_rising_edge();

            match select(brake_fut, drive_fut).await {
                First(_) => speedom_led.toggle(),
                Second(_) => enable_led.toggle(),
            }
        }
    };

    let potentiometer = async {
        loop {
            match ring.read(&mut samples).await {
                Ok(n) if n > 0 => {
                    // Newest reading in the batch, no averaging
                    let raw = samples[n - 1] as u32;

                    // 12-bit ADC: 0..=4095 -> 0..=100%. Multiply before dividing.
                    let pct = raw * 100 / 4095;
                    info!("pedal: {}%", pct);
                }
                Ok(_) => {}
                Err(_) => info!("ADC ring buffer overrun"),
            }
        }
    };

    join(buttons, potentiometer).await;
    

    // Init IWDG
    //let mut alt = false;

    // Loop for petting IWDG
    // loop {
    //     if !alt {
    //         info!(".");
    //     } else {
    //         info!("..");
    //     }
    //     alt = !alt;
    //     Timer::after_millis(100).await;
    // }
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
