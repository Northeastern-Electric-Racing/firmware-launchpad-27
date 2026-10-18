#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::exti::InterruptHandler;
use embassy_stm32::gpio::{Level, Speed};
use embassy_stm32::gpio::Pull::{self, Up};
use embassy_stm32::peripherals::{DMA2, EXTI8, EXTI9};
use embassy_stm32::timer::low_level::OutputCompareMode::Toggle;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use embassy_stm32::adc::{Adc, AdcChannel, CONTINUOUS, RingBufferedAdc, SampleTime};

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
    let mut direction_led = embassy_stm32::gpio::Output::new(p.PC8, Level::High, Speed::Low);
    let mut acceleration_led = embassy_stm32::gpio::Output::new(p.PC9, Level::High, Speed::Low);
    
    // TODO: Initialize button inputs
    let mut drive_button = embassy_stm32::exti::ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
    let mut brake_button = embassy_stm32::exti::ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
    let adc = embassy_stm32::adc::Adc::new(p.ADC1);
    let adc_pin = p.PA3.degrade_adc();
    let mut dma_buf = [0u16; 16];
    let mut measurements = [0u16; 8];

    //configure into_ring_buffered()
    let mut adc = adc.into_ring_buffered(
        p.DMA2_CH0,
        &mut dma_buf,
        AdcIrqs,
        [(adc_pin, SampleTime::CYCLES15)].into_iter(),
        CONTINUOUS,
        embassy_stm32::adc::Exten::DISABLED,
    );

    adc.start();
    loop {
    match select(
        adc.read(&mut measurements),
        select(
            drive_button.wait_for_falling_edge(),
            brake_button.wait_for_falling_edge(),
        ),
    ).await {
        Either::First(result) => {
            match result {
                Ok(_count) => {
                    let reading = measurements[0];
                    let percentage =
                        (reading as u32 * 100) / 4095;

                    info!("Pedal: {}%", percentage);
                }
                Err(_error) => {
                    info!("ADC buffer overrun!");
                }
            }
        }

        Either::Second(Either::First(_)) => {
            direction_led.toggle();
        }

        Either::Second(Either::Second(_)) => {
            acceleration_led.toggle();
        }
        }
    }


}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
