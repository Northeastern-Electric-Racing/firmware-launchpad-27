#![no_std]
#![no_main]

#[cfg(not(feature = "blink"))]
mod button;
#[cfg(not(any(feature = "blink", feature = "buttons")))]
mod car;
#[cfg(not(feature = "blink"))]
mod input;
#[cfg(not(any(feature = "blink", feature = "buttons")))]
mod led;
#[cfg(not(any(feature = "blink", feature = "buttons")))]
mod minicar;
#[cfg(all(not(feature = "blink"), any(feature = "adc", not(feature = "buttons"))))]
mod potentiometer;
#[cfg(not(any(feature = "blink", feature = "buttons")))]
mod temp;

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use embassy_executor::Spawner;
use embassy_stm32::exti::InterruptHandler;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{Config, bind_interrupts, dma, interrupt, peripherals};
#[cfg(not(feature = "blink"))]
use embassy_stm32::{exti::ExtiInput, gpio::Pull};
#[cfg(feature = "blink")]
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct ExtiIrqs {
    EXTI9_5 => InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => InterruptHandler<interrupt::typelevel::EXTI15_10>;
});
bind_interrupts!(struct I2cIrqs {
    DMA1_CHANNEL2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
    DMA1_CHANNEL7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
});
bind_interrupts!(struct AdcIrqs {
    DMA2_CHANNEL1 => dma::InterruptHandler<peripherals::DMA2_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    // G474 ADC1/2 needs an explicit kernel clock; reset default is disabled.
    config.rcc.mux.adc12sel = embassy_stm32::rcc::mux::Adcsel::SYS;
    let p = embassy_stm32::init(config);
    // *** FIRMWARE LAUNCHPAD: START HERE! ***
    // info!("Welcome to Firmware Launchpad!");
    // TODO: Initialize LED outputs

    // TODO: Initialize button inputs
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);
    #[cfg(feature = "blink")]
    {
        let mut enable_led = enable_led;
        let mut speedometer_led = speedometer_led;
        defmt::info!("Phase 1 blink check: both LEDs toggle every 500 ms");
        loop {
            Timer::after_millis(500).await;
            enable_led.toggle();
            speedometer_led.toggle();
        }
    }
    #[cfg(all(feature = "buttons", not(feature = "blink")))]
    {
        let drive = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
        let brake = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
        let reverse = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
        let mut enable_led = enable_led;
        let mut speedometer_led = speedometer_led;
        defmt::info!(
            "Button test: released levels Drive={} Brake={} Reverse={} (expect true)",
            drive.is_high(),
            brake.is_high(),
            reverse.is_high(),
        );
        _spawner.spawn(button::button_task(reverse, drive, brake).unwrap());
        #[cfg(feature = "adc")]
        {
            _spawner.spawn(potentiometer::potentiometer_task(p.ADC1, p.PA0, p.DMA2_CH1).unwrap());
            defmt::info!("ADC test: potentiometer on A0/PA0 (ADC1 channel 1), 0..4095 -> 0..100%");
        }
        defmt::info!("Button test ready: Drive -> Enable; Brake -> Speedometer; Reverse -> log");
        let ready_since_ms = embassy_time::Instant::now().as_millis();
        loop {
            let press = button::BUTTON_EVENTS.receive().await;
            if !press.is_fresh(ready_since_ms) {
                continue;
            }
            match press.kind {
                button::ButtonType::Accelerate => {
                    enable_led.toggle();
                    defmt::info!("Drive pressed: Enable toggled");
                }
                button::ButtonType::Brake => {
                    speedometer_led.toggle();
                    defmt::info!("Brake pressed: Speedometer toggled");
                }
                button::ButtonType::Direction => defmt::info!("Reverse pressed"),
            }
        }
    }
    #[cfg(not(any(feature = "blink", feature = "buttons")))]
    {
        // p2: DRIVE/PA8, BRAKE/PA9, REVERSE/PA10. Pressed = ground.
        let drive = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, ExtiIrqs);
        let brake = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, ExtiIrqs);
        let reverse = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
        // NUCLEO-G474RE Arduino I2C header: D15/PB8=SCL, D14/PB9=SDA.
        // External I2C pull-ups required. PB8 is SCL, not sensor reset.
        let mut i2c_config = embassy_stm32::i2c::Config::default();
        i2c_config.timeout = embassy_time::Duration::from_millis(100);
        let i2c = embassy_stm32::i2c::I2c::new(
            p.I2C1, p.PB8, p.PB9, p.DMA1_CH7, // Transmit.
            p.DMA1_CH2, // Receive.
            I2cIrqs, i2c_config,
        );
        _spawner.spawn(led::led_task(enable_led, speedometer_led).unwrap());
        _spawner.spawn(minicar::state_machine_task().unwrap());
        _spawner.spawn(button::button_task(reverse, drive, brake).unwrap());
        _spawner.spawn(potentiometer::potentiometer_task(p.ADC1, p.PA0, p.DMA2_CH1).unwrap());
        _spawner.spawn(temp::temp_task(i2c).unwrap());
        defmt::info!("Phase 4 tasks started; motion locked until a valid cool temperature");
        core::future::pending::<()>().await;
    }
}
#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
