#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::{info,Format};
use embassy_executor::Spawner;
use embassy_futures::select::Either::{self, First, Second};
use embassy_futures::select::{Either3, select, select3};
use embassy_stm32::Peri;
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::mode::Async;
use embassy_stm32::{
    Config, bind_interrupts, dma, interrupt,
    peripherals::{self},
};
use embassy_stm32::adc::{Adc, Exten, AdcChannel, CONTINUOUS, SampleTime};
use embassy_stm32::gpio::{Level, Output, Speed, Pull};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use defmt::unwrap;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

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

const MAX_SPEED: i32 = 100;
const BRAKE_STEP: i32 = 20;

static BUTTONS: Channel<CriticalSectionRawMutex, Button, 4> = Channel::new();
static PEDAL_PCT: Signal<CriticalSectionRawMutex, u8> = Signal::new();
static BLINK_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();

#[derive(Clone, Copy, Format)]
enum Button {
    Drive,
    Brake,
    Reverse,
}

#[derive(Clone, Copy, Format)]
enum Gear {
    Gear1,
    Gear2,
    Gear3,
    Gear4,
}

impl Gear {
    fn from_percent(pct: u8) -> Self {
        match pct {
            0..=24 => Gear::Gear1,
            25..=49 => Gear::Gear2,
            50..=74 => Gear::Gear3,
            _ => Gear::Gear4,
        }
    }
    fn step(self) -> i32 {
        match self {
            Gear::Gear1 => 5,
            Gear::Gear2 => 10,
            Gear::Gear3 => 20,
            Gear::Gear4 => 35,
        }
    }
}

struct MiniCar {
    velocity: i32,
    gear: Gear,
    reverse: bool,
}
 
impl MiniCar {
    fn new() -> Self {
        Self {
            velocity: 0,
            gear: Gear::Gear1,
            reverse: false,
        }
    }
    fn on_button(&mut self, button: Button) {
        match button {
            Button::Drive => {
                let step = self.gear.step();
                let delta = if self.reverse {-step} else {step};
                self.velocity = (self.velocity + delta).clamp(-MAX_SPEED, MAX_SPEED);
            }
            Button::Brake => {
                self.velocity = 
                if self.velocity > 0 {
                    (self.velocity - BRAKE_STEP).max(0)
                } 
                else {
                    (self.velocity + BRAKE_STEP).min(0)
                };
            }
            Button::Reverse => {
                self.reverse = !self.reverse;
                self.velocity = -self.velocity;
            }
        }
    }
    fn blink_ms(&self) -> u64 {
        match self.velocity.abs() {
            0..=24 => 2500,
            25..=49 => 2000,
            50..=74 => 1000,
            _ => 500,
        }
    }
}

// #[embassy_executor::task]
// async fn buttons_task(
//     mut drive: ExtiInput<'static, Async>,
//     mut brake: ExtiInput<'static, Async>,
//     mut reverse: ExtiInput<'static, Async>,
// ) {
//     loop {
//         let button = match select3(
//             drive.wait_for_rising_edge(),
//             brake.wait_for_rising_edge(),
//             reverse.wait_for_rising_edge(),
//         )
//         .await
//         {
//             Either3::First(_) => Button::Drive,
//             Either3::Second(_) => Button::Brake,
//             Either3::Third(_) => Button::Reverse,
//         };
 
//         BUTTONS.send(button).await;
//         Timer::after_millis(30).await;
//     }
// }

// #[embassy_executor::task]
// async fn pedal_task(
//     adc: Peri<'static, peripherals::ADC1>,
//     pin: Peri<'static, peripherals::PA3>,
//     dma: Peri<'static, peripherals::DMA2_CH0>,
// ) {
//     let adc = Adc::new(adc);
//     let pedal = pin.degrade_adc();
//     let mut dma_buf = [0u16; 256];
//     let mut ring = adc.into_ring_buffered(
//         dma,
//         &mut dma_buf,
//         AdcIrqs,
//         [(pedal, SampleTime::CYCLES144)].into_iter(),
//         CONTINUOUS,
//         Exten::DISABLED,
//     );

//     ring.start();

//     let mut samples = [0u16; 8];
//     let mut last_pct = u8::MAX;
 
//     loop {
//         match ring.read(&mut samples).await {
//             Ok(n) if n > 0 => {
//                 // Average the batch, then convert to percent.
//                 let sum: u32 = samples[..n].iter().map(|&s| s as u32).sum();
//                 let avg = sum / n as u32;
//                 let pct = avg * 100 / 4095; 
//                 info!("pedal: {}%", pct);
//             }
//             Ok(_) => {}
//             Err(_) => info!("ADC ring buffer overrun"),
//         }
//         Timer::after_millis(300).await;   
//     }
// }

// #[embassy_executor::task]
// async fn speedometer_task(mut led: Output<'static>) {
//     let mut period = Duration::from_millis(2500);
//     loop {
//         match select(BLINK_MS.wait(), Timer::after(period)).await {
//             Either::First(ms) => period = Duration::from_millis(ms),
//             Either::Second(_) => led.toggle(),
//         }
//     }
// }

// #[embassy_executor::task]
// async fn car_task() {
//     let mut car = MiniCar::new();
//     BLINK_MS.signal(car.blink_ms());
 
//     loop {
//         match select(BUTTONS.receive(), PEDAL_PCT.wait()).await {
//             Either::First(button) => car.on_button(button),
//             Either::Second(pct) => car.gear = Gear::from_percent(pct),
//         }
//         BLINK_MS.signal(car.blink_ms());
//         info!(
//             "gear: {}, velocity: {} m/s, reverse: {}",
//             car.gear, car.velocity, car.reverse
//         );
//     }
// }

#[embassy_executor::task]
async fn buttons_task(
    mut brake_button: ExtiInput<'static, Async>,
    mut drive_button: ExtiInput<'static, Async>,
    mut speedom_led: Output<'static>,
    mut enable_led: Output<'static>,
) {
    loop {
        let brake_fut = brake_button.wait_for_any_edge();
        let drive_fut = drive_button.wait_for_any_edge();

        match select(brake_fut, drive_fut).await {
            First(_) => speedom_led.set_level(brake_button.is_high().into()),
            Second(_) => enable_led.set_level(drive_button.is_high().into()),
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
 
    let mut dma_buf = [0u16; 256];
 
    let mut ring = adc.into_ring_buffered(
        dma,
        &mut dma_buf,
        AdcIrqs,
        [(pedal, SampleTime::CYCLES144)].into_iter(),
        CONTINUOUS,
        Exten::DISABLED,
    );
 
    let mut samples = [0u16; 128]; // must be exactly half of dma_buf
 
    loop {
        match ring.read(&mut samples).await {
            Ok(n) if n > 0 => {
                // Average the batch, then convert to percent.
                let sum: u32 = samples[..n].iter().map(|&s| s as u32).sum();
                let avg = sum / n as u32;
                let pct = avg * 100 / 4095; 
                info!("pedal: {}%", pct);
            }
            Ok(_) => {}
            Err(_) => info!("ADC ring buffer overrun"),
        }
    Timer::after_millis(300).await;
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
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, ExtiIrqs);
 
    // spawner.spawn(unwrap!(buttons_task(drive_button, brake_button, reverse_button)));
    // spawner.spawn(unwrap!(pedal_task(p.ADC1, p.PA3, p.DMA2_CH0)));
    // spawner.spawn(unwrap!(speedometer_task(speedom_led)));
    // spawner.spawn(unwrap!(car_task()));
 
    spawner.spawn(unwrap!(buttons_task( brake_button, drive_button, speedom_led,enable_led)));
    spawner.spawn(unwrap!(pedal_task(p.ADC1, p.PA3, p.DMA2_CH0)));
}

#[exception]
unsafe fn HardFault(_info: &ExceptionFrame) -> ! {
    SCB::sys_reset(); // reset STM
}
