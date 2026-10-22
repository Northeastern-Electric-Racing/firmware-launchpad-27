#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::adc::AdcChannel;
use embassy_stm32::adc::Exten;
use embassy_stm32::adc::SampleTime;
use embassy_stm32::adc::CONTINUOUS;
use embassy_stm32::Peri;

use embassy_stm32::exti::{self, ExtiInput, InterruptHandler};
use embassy_stm32::gpio::Pull::Up;
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_time::Timer;
use embassy_futures::select::{select, Either};
use {defmt_rtt as _, panic_probe as _};
use embassy_stm32::gpio::{Output,Level,Speed};

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
    let  enable_led: Output<'_> =  Output::new(p.PC8, Level::High, Speed::Low);
    let  speedometer_led: Output<'_> = Output::new(p.PC9, Level::High, Speed::Low);
    
    // TODO: Initialize button inputs

    let  drive = exti::ExtiInput::new(p.PA8,p.EXTI8,Up,ExtiIrqs);
    let  brake = exti::ExtiInput::new(p.PA9,p.EXTI9,Up,ExtiIrqs);

    spawner.spawn(buttons(drive,brake,enable_led,speedometer_led).unwrap());

    spawner.spawn(adc(p.ADC1, p.DMA2_CH0, p.PA3).unwrap());
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

#[embassy_executor::task]
async fn blink(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn buttons(
    mut drive: ExtiInput<'static,Async>,
    mut brake: ExtiInput<'static,Async>,
    mut enable: Output<'static>,
    mut speedometer: Output<'static>,
) {
    loop{
        match select(drive.wait_for_falling_edge(), brake.wait_for_falling_edge()).await {
            Either::First(_) => enable.set_high(),
            Either::Second(_) => speedometer.set_high(),
        };
    }
}

#[embassy_executor::task]
async fn adc(
    adc_per: Peri<'static, peripherals::ADC1>,
    dma_per: Peri<'static, peripherals::DMA2_CH0>,
    pot_pin: Peri<'static, peripherals::PA3>,
) {
    
    let adc: Adc<'_, peripherals::ADC1> = Adc::new(adc_per);
    
    let mut adc_buf: [u16; 128]=  [0u16;128];
    
    let mut ring_adc = adc.into_ring_buffered(
        dma_per,
        &mut adc_buf,
        AdcIrqs,
        [(pot_pin.degrade_adc(), SampleTime::CYCLES480)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED
        );
    
    let mut measurements : [u16;128] = [0u16; 128];  
        
    loop {
    match ring_adc.read(&mut measurements).await {
        Ok(_) => {
            let raw = measurements[0];
            let percent = (raw as u32) * 100 / 4095;
            info!("pedal: {}%", percent);
        }
        Err(_) => { info!("adc read error"); }
    }
}
}
