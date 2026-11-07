#![no_std]
#![no_main]

mod button;
mod led;
mod minicar;
mod potentiometer;
mod temp;

use cortex_m::peripheral::SCB;
use cortex_m_rt::ExceptionFrame;
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::adc::AdcChannel;
use embassy_stm32::adc::AdcConfig;
use embassy_stm32::adc::Exten;
use embassy_stm32::adc::SampleTime;
use embassy_stm32::adc::CONTINUOUS;

use embassy_stm32::exti::{self, InterruptHandler};
use embassy_stm32::gpio::Pull::Up;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use embassy_stm32::gpio::{Output,Level,Speed};

use button::button_task;
use led::led_task;
use minicar::state_machine_task;
use potentiometer::potentiometer_task;

bind_interrupts!(struct ExtiIrqs {
    EXTI9_5 => InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

bind_interrupts!(struct I2cIrqs {
    DMA1_CHANNEL2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
    DMA1_CHANNEL7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
    I2C2_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C2>;
});

bind_interrupts!(struct AdcIrqs {
    DMA1_CHANNEL1 => dma::InterruptHandler<peripherals::DMA1_CH1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Create a config for STM32F4
    let config = Config::default();

    // intialize peripherals
    let p = embassy_stm32::init(config);

    // *** FIRMWARE LAUNCHPAD: START HERE! ***
    info!("Welcome to Firmware Launchpad!");

    // Initialize LED outputs
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // Initialize button inputs
    let drive = exti::ExtiInput::new(p.PA8, p.EXTI8, Up, ExtiIrqs);
    let brake = exti::ExtiInput::new(p.PA9, p.EXTI9, Up, ExtiIrqs);
    let reverse = exti::ExtiInput::new(p.PA10, p.EXTI10, Up, ExtiIrqs);

    // Initialize the potentiometer ADC; the DMA buffer must live for the whole program
    let adc = Adc::new(p.ADC1, AdcConfig::default());
    let adc_buf = cortex_m::singleton!(: [u16; 128] = [0u16; 128]).unwrap();
    let ring_adc = adc.into_ring_buffered(
        p.DMA1_CH1,
        adc_buf,
        AdcIrqs,
        [(p.PA0.degrade_adc(), SampleTime::CYCLES2_5)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED,
    );

    // Spawn Phase 3 tasks
    spawner.spawn(button_task(reverse, drive, brake).unwrap());
    spawner.spawn(potentiometer_task(ring_adc).unwrap());
    spawner.spawn(state_machine_task().unwrap());
    spawner.spawn(led_task(enable_led, speedometer_led).unwrap());

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

// Keep the handler independent of the cortex-m-rt exception proc-macro.
#[unsafe(no_mangle)]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
