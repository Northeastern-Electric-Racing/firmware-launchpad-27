#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info};
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, AdcChannel, CONTINUOUS, Exten, SampleTime};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
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
async fn main(_spawner: Spawner) {
    // Create a config for STM32F4
    let config = Config::default();

    // intialize peripherals
    let p = embassy_stm32::init(config);

    // *** FIRMWARE LAUNCHPAD: START HERE! ***
    info!("Welcome to Firmware Launchpad!");

    // Initialize LED outputs
    let mut _enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let mut _speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // Initialize button inputs
    let mut _drive_button  = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let mut _brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    //Init ADC and DMA
    const ADC_SAMPLE: usize = 16;
    const MAX_RESOLUTION: f32 = 4095.0;
    let adc = Adc::new(p.ADC1);
    let mut dma_adc_buf = [0u16; ADC_SAMPLE*2];
    let mut measurements = [0u16; ADC_SAMPLE*2];

    let mut adc = adc.into_ring_buffered(
        p.DMA2_CH0, 
        &mut dma_adc_buf,
        AdcIrqs,
        [(p.PA3.degrade_adc(), SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED
    );


    // Init IWDG
    let mut _alt = false;

    // Loop for petting IWDG
    loop {
        adc.clear();
        match adc.read(&mut measurements).await {
            Ok(_)=> {
                let mut average: f32 = 0.0;
                for i in 1..ADC_SAMPLE{
                    average = average + (measurements[i] as f32 / MAX_RESOLUTION);
                }
                average = average / (ADC_SAMPLE as f32);
                info!("Potentiometer at {}%", average*100.0);
            }
            Err(e) => {
                info!("Error {} occured", e);
            }
        }
        
    }
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
