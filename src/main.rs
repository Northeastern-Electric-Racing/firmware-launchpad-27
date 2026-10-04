#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::Either::{First, Second};
use embassy_futures::select::select;
use embassy_stm32::Peri;
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_stm32::adc::{Adc, Exten, AdcChannel, CONTINUOUS, SampleTime};
use embassy_stm32::gpio::{Level, Output, Speed, Pull};
use {defmt_rtt as _, panic_probe as _};
use defmt::unwrap;

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
    loop {
        let brake_fut = brake_button.wait_for_rising_edge();
        let drive_fut = drive_button.wait_for_rising_edge();
 
        match select(brake_fut, drive_fut).await {
            First(_) => speedom_led.toggle(),
            Second(_) => enable_led.toggle(),
        }
    }
}

#[embassy_executor::task]
async fn pedal_task(
    adc: Peri<'static, peripherals::ADC1>,
    pin: Peri<'static, peripherals::PA3>,
    dma: Peri<'static, peripherals::DMA2_CH0>,
) {
    let adc = Adc::new(adc);
    let pedal = pin.degrade_adc();
 
    // Lives inside this task's future, so it stays valid as long as the task runs.
    let mut dma_buf = [0u16; 256];
 
    let mut ring = adc.into_ring_buffered(
        dma,
        &mut dma_buf,
        AdcIrqs,
        [(pedal, SampleTime::CYCLES144)].into_iter(),
        CONTINUOUS,      // free-running conversions
        Exten::DISABLED, // ignored for CONTINUOUS, but still required
    );
 
    let mut samples = [0u16; 128]; // must be exactly half of dma_buf
 
    loop {
        match ring.read(&mut samples).await {
            Ok(n) if n > 0 => {
                // Newest reading. 12-bit ADC: 0..=4095 -> 0..=100%.
                let raw = samples[n - 1] as u32;
                let pct = raw * 100 / 4095;
                info!("pedal: {}%", pct);
            }
            Ok(_) => {}
            Err(_) => info!("ADC ring buffer overrun"),
        }
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
 
    spawner.spawn(unwrap!(buttons_task(
        brake_button,
        drive_button,
        speedom_led,
        enable_led
    )));
    spawner.spawn(unwrap!(pedal_task(
        p.ADC1, 
        p.PA3, 
        p.DMA2_CH0
    )));
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
