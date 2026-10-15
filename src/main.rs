#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_futures::select::{select, Either};
use embassy_stm32::adc::{
    Adc,
    AdcChannel,
    Exten,
    SampleTime,
    CONTINUOUS,
};
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

const SAMPLES: usize = 16;
const MAX_TWELVE_BIT_RESOLUTION: f32 = 4095.0;

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
    let mut speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // TODO: Initialize button inputs
    let mut drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let mut brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    // loop {
    //     let button_pressed = select(drive_button.wait_for_any_edge(), brake_button.wait_for_any_edge()).await;
    //     match button_pressed {
    //         Either::First(_) => {
    //             if drive_button.is_high() {
    //                 enable_led.set_high();
    //             } else {
    //                 enable_led.set_low();
    //             }
    //         }

    //         Either::Second(_) => {
    //             if brake_button.is_high() {
    //                 speedometer_led.set_high();
    //             } else {
    //                 speedometer_led.set_low();
    //             }
    //         }
    //     }
    // }

    //phase 2 
    let adc = Adc::new(p.ADC1);
    let adc_pin = p.PA3.degrade_adc();

    let mut dma_buffer = [0u16; SAMPLES * 2];
    
    let mut adc = adc.into_ring_buffered(
        p.DMA2_CH0, 
        &mut dma_buffer, 
        AdcIrqs,  
        [(adc_pin, SampleTime::CYCLES480)].into_iter(), 
        CONTINUOUS,
        Exten::DISABLED);

    let mut measurements = [0u16; SAMPLES];

    loop {
        adc.clear();
        match adc.read(&mut measurements).await {
            Ok(_) => {
                let mut sum: u32 = 0;
                for measurement in measurements {
                    sum += measurement as u32;
                }

                let avg = sum / SAMPLES as u32;

                let percentage = avg as f32 / MAX_TWELVE_BIT_RESOLUTION * 100.0;

                info!("ADC reading: {} ({}%)", avg, percentage);
            }

            Err(e) => {
                warn!("ADC DMA overrun: {}", e);
            }
        }

        Timer::after_millis(100).await;
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
