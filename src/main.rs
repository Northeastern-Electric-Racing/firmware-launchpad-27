#![no_std]
#![no_main]

mod button;
mod potentiometer;
mod minicar;
mod led;
//mod temp;

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
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

use static_cell::StaticCell;

use button::button_task;
use potentiometer::potentiometer_task;
use minicar::state_machine_task;
use led::led_task;

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

static DMA_BUFFER: StaticCell<[u16; SAMPLES * 2]> = StaticCell::new();

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
    let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);

    //phase 2 
    let adc = Adc::new(p.ADC1);
    let adc_pin = p.PA3.degrade_adc();

    let dma_buffer = DMA_BUFFER.init([0u16; SAMPLES * 2]);
    
    let adc = adc.into_ring_buffered(
        p.DMA2_CH0,
        dma_buffer,
        AdcIrqs,
        [(adc_pin, SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED
    );

    spawner.spawn(button_task(reverse_button, drive_button, brake_button).unwrap());

    spawner.spawn(potentiometer_task(adc).unwrap());

    spawner.spawn(led_task(enable_led, speedometer_led).unwrap());

    spawner.spawn(state_machine_task().unwrap());

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
