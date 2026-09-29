#![no_std]
#![no_main]

use core::panic::PanicInfo;

use cortex_m::peripheral::SCB;
use defmt::{Format, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_stm32::adc::{self, Adc, AdcChannel, SampleTime};
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::i2c::{self, I2c, Master};
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::{ADC1, DMA1_CH2, DMA1_CH7, DMA2_CH0, I2C2, PA3};
use embassy_stm32::{Config, Peri, bind_interrupts, dma, interrupt};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Duration, Timer};
use embedded_sht3x::{DEFAULT_I2C_ADDRESS, Sht3x};

// Drive (PA8) and Brake (PA9) share EXTI9_5; Reverse (PA10) is on EXTI15_10
bind_interrupts!(struct Irqs {
    EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
    EXTI15_10 => exti::InterruptHandler<interrupt::typelevel::EXTI15_10>;
    DMA2_STREAM0 => dma::InterruptHandler<DMA2_CH0>;
    // SHT30 temperature sensor is on I2C2, using DMA1 stream 7 (TX) and stream 2 (RX)
    I2C2_EV => i2c::EventInterruptHandler<I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<I2C2>;
    DMA1_STREAM7 => dma::InterruptHandler<DMA1_CH7>;
    DMA1_STREAM2 => dma::InterruptHandler<DMA1_CH2>;
});

// 3.3V reference voltage, 4095 is max, 12 bit
const ADC_MAX: u32 = 4095;

// DMA fills this constantly; each read takes half of it (DMA_BUF_LEN / 2 samples)
const DMA_BUF_LEN: usize = 256;

const MAX_SPEED: i32 = 100;
const BRAKE_DECREASE: i32 = 20;

// fault above this, clear below it
const FAULT_TEMPERATURE_C: f32 = 35.0;
const TEMPERATURE_SAMPLE_PERIOD_MS: u64 = 500;

#[derive(Clone, Copy, Format)]
enum Button {
    Drive,
    Brake,
    Reverse,
}

#[derive(Clone, Copy, PartialEq, Format)]
enum Gear {
    Gear1,
    Gear2,
    Gear3,
    Gear4,
}

impl Gear {
    fn from_percent(percent: u32) -> Self {
        match percent {
            0..25 => Gear::Gear1,
            25..50 => Gear::Gear2,
            50..75 => Gear::Gear3,
            _ => Gear::Gear4,
        }
    }

    /// velocity increase (m/s) per drive press
    fn increment(self) -> i32 {
        match self {
            Gear::Gear1 => 5,
            Gear::Gear2 => 10,
            Gear::Gear3 => 20,
            Gear::Gear4 => 35,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Format)]
enum Direction {
    Forward,
    Reverse,
}

/// all state of the car
#[derive(Format)]
struct MiniCar {
    /// m/s, negative when moving backwards
    velocity: i32,
    gear: Gear,
    direction: Direction,
    /// overheated: speed held at 0 and buttons ignored until it cools down
    faulted: bool,
}

impl MiniCar {
    const fn new() -> Self {
        Self {
            velocity: 0,
            gear: Gear::Gear1,
            direction: Direction::Forward,
            faulted: false,
        }
    }

    fn set_fault(&mut self, faulted: bool) {
        self.faulted = faulted;
        if faulted {
            self.velocity = 0;
        }
    }

    fn drive(&mut self) {
        let increment = match self.direction {
            Direction::Forward => self.gear.increment(),
            Direction::Reverse => -self.gear.increment(),
        };
        self.velocity = (self.velocity + increment).clamp(-MAX_SPEED, MAX_SPEED);
    }

    /// slows down toward 0 m/s, whichever way the car is moving
    fn brake(&mut self) {
        if self.velocity > 0 {
            self.velocity = (self.velocity - BRAKE_DECREASE).max(0);
        } else {
            self.velocity = (self.velocity + BRAKE_DECREASE).min(0);
        }
    }

    fn reverse(&mut self) {
        self.direction = match self.direction {
            Direction::Forward => Direction::Reverse,
            Direction::Reverse => Direction::Forward,
        };
    }

    fn speed(&self) -> i32 {
        self.velocity.abs()
    }

    /// how long the speedometer LED stays in each state, or None to hold it off
    fn speedometer_toggle_period(&self) -> Option<Duration> {
        if self.faulted {
            return None;
        }
        Some(match self.speed() {
            0..25 => Duration::from_millis(2500),
            25..50 => Duration::from_millis(2000),
            50..75 => Duration::from_millis(1000),
            _ => Duration::from_millis(500),
        })
    }
}

// inter task communication
static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, Button> = Signal::new();
static POT_PERCENT_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();
static TOGGLE_PERIOD_SIGNAL: Signal<CriticalSectionRawMutex, Option<Duration>> = Signal::new();
static TEMPERATURE_FAULT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

/// waits for button presses and tells the control task which one was pressed
#[embassy_executor::task]
async fn button_task(
    mut drive: ExtiInput<'static, Async>,
    mut brake: ExtiInput<'static, Async>,
    mut reverse: ExtiInput<'static, Async>,
) {
    loop {
        // buttons short to GND when pressed, so a press is a falling edge
        let button = match select3(
            drive.wait_for_falling_edge(),
            brake.wait_for_falling_edge(),
            reverse.wait_for_falling_edge(),
        )
        .await
        {
            Either3::First(()) => Button::Drive,
            Either3::Second(()) => Button::Brake,
            Either3::Third(()) => Button::Reverse,
        };
        BUTTON_SIGNAL.signal(button);

        // Debounce: ignore contact bounce right after a press
        Timer::after_millis(50).await;
    }
}

/// samples the potentiometer over DMA and sends out its position as a percentage
#[embassy_executor::task]
async fn potentiometer_task(
    adc: Peri<'static, ADC1>,
    pin: Peri<'static, PA3>,
    dma: Peri<'static, DMA2_CH0>,
) {
    let mut dma_buf = [0u16; DMA_BUF_LEN];
    let mut ring_adc = Adc::new(adc).into_ring_buffered(
        dma,
        &mut dma_buf,
        Irqs,
        [(pin.degrade_adc(), SampleTime::CYCLES480)].into_iter(),
        adc::CONTINUOUS,
        adc::Exten::DISABLED,
    );

    let mut samples = [0u16; DMA_BUF_LEN / 2];
    let mut last_percent = u32::MAX;
    loop {
        match ring_adc.read(&mut samples).await {
            Ok(_) => {
                // Average the batch to smooth out noise
                let avg = samples.iter().map(|&s| s as u32).sum::<u32>() / samples.len() as u32;
                let percent = avg * 100 / ADC_MAX;

                // Samples arrive thousands of times per second, so only send changes
                if percent != last_percent {
                    POT_PERCENT_SIGNAL.signal(percent);
                    last_percent = percent;
                }
            }
            // DMA lapped us; the next read restarts sampling
            Err(_) => defmt::warn!("ADC overrun"),
        }
    }
}

/// reads the SHT30 over I2C and tells the control task when the car over/underheats
#[embassy_executor::task]
async fn temperature_task(i2c: I2c<'static, Async, Master>, _n_reset: Output<'static>) {
    // _n_reset is held high (sensor out of reset) for as long as this task owns it.
    // The SHT30 needs up to 1 ms after power-up/reset before it accepts commands.
    Timer::after_millis(2).await;

    // driver converts the raw reading to Celsius per the datasheet (section 4.13):
    // T = -45 + 175 * raw / (2^16 - 1)
    let mut sensor = match Sht3x::new(i2c, DEFAULT_I2C_ADDRESS, Delay).await {
        Ok(sensor) => sensor,
        Err(e) => {
            // Fail safe: without a working sensor we can't know the car isn't overheating
            defmt::error!(
                "SHT30 init failed, faulting car: {}",
                defmt::Debug2Format(&e)
            );
            TEMPERATURE_FAULT_SIGNAL.signal(true);
            return;
        }
    };

    let mut faulted = false;
    loop {
        let fault = match sensor.single_measurement().await {
            Ok(measurement) => {
                let celsius = measurement.temperature.0;
                info!("Temperature: {} C", celsius);
                if celsius > FAULT_TEMPERATURE_C {
                    true
                } else if celsius < FAULT_TEMPERATURE_C {
                    false
                } else {
                    faulted
                }
            }
            Err(e) => {
                defmt::warn!(
                    "SHT30 read failed, faulting car: {}",
                    defmt::Debug2Format(&e)
                );
                true
            }
        };

        if fault != faulted {
            faulted = fault;
            TEMPERATURE_FAULT_SIGNAL.signal(fault);
        }

        Timer::after_millis(TEMPERATURE_SAMPLE_PERIOD_MS).await;
    }
}

/// blinks the speedometer LED, picking up new toggle periods as they arrive
#[embassy_executor::task]
async fn speedometer_task(mut led: Output<'static>) {
    let mut period = MiniCar::new().speedometer_toggle_period();
    loop {
        match period {
            Some(toggle_period) => {
                led.toggle();

                // Apply a new period right away instead of finishing a long 2.5 s wait
                if let Either::Second(new_period) =
                    select(Timer::after(toggle_period), TOGGLE_PERIOD_SIGNAL.wait()).await
                {
                    period = new_period;
                }
            }
            // Faulted: hold the LED off until blinking is allowed again
            None => {
                led.set_low();
                period = TOGGLE_PERIOD_SIGNAL.wait().await;
            }
        }
    }
}

/// top level logic: updates the MiniCar from button presses and pedal position
#[embassy_executor::task]
async fn control_task(mut direction_led: Output<'static>) {
    let mut car = MiniCar::new();
    let mut toggle_period = car.speedometer_toggle_period();

    loop {
        match select3(
            BUTTON_SIGNAL.wait(),
            POT_PERCENT_SIGNAL.wait(),
            TEMPERATURE_FAULT_SIGNAL.wait(),
        )
        .await
        {
            Either3::First(button) if car.faulted => {
                info!("{} ignored: car is faulted", button);
            }
            Either3::First(button) => {
                match button {
                    Button::Drive => car.drive(),
                    Button::Brake => car.brake(),
                    Button::Reverse => car.reverse(),
                }
                info!("{} pressed: {}", button, car);
            }
            Either3::Second(percent) => {
                let gear = Gear::from_percent(percent);
                if gear != car.gear {
                    car.gear = gear;
                    info!("Pedal at {}%: {}", percent, car);
                }
            }
            Either3::Third(faulted) => {
                car.set_fault(faulted);
                if faulted {
                    defmt::warn!("Overheat fault: {}", car);
                } else {
                    info!("Fault cleared: {}", car);
                }
            }
        }

        // enable LED sits on the DIRECTION net: on while in reverse
        direction_led.set_level(match car.direction {
            Direction::Forward => Level::Low,
            Direction::Reverse => Level::High,
        });

        // only send real changes: every signal restarts the blink timer
        if car.speedometer_toggle_period() != toggle_period {
            toggle_period = car.speedometer_toggle_period();
            TOGGLE_PERIOD_SIGNAL.signal(toggle_period);
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Create a config for STM32F4
    let config = Config::default();

    // intialize peripherals
    let p = embassy_stm32::init(config);

    info!("Welcome to Firmware Launchpad!");

    // LEDs are wired pin -> LED -> resistor -> GND, so driving high turns them on
    let enable_led = Output::new(p.PC8, Level::Low, Speed::Low);
    let speedometer_led = Output::new(p.PC9, Level::Low, Speed::Low);

    // buttons short the pin to GND when pressed, so pull up
    let drive_button = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up, Irqs);
    let brake_button = ExtiInput::new(p.PA9, p.EXTI9, Pull::Up, Irqs);
    let reverse_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Up, Irqs);

    spawner.spawn(button_task(drive_button, brake_button, reverse_button).unwrap());
    // pot (pedal sensor) is on PA3, which is ADC1 channel 3
    spawner.spawn(potentiometer_task(p.ADC1, p.PA3, p.DMA2_CH0).unwrap());
    spawner.spawn(speedometer_task(speedometer_led).unwrap());
    spawner.spawn(control_task(enable_led).unwrap());

    // SHT30 on I2C2 (SCL PB10, SDA PB11); the board has 4.7k pull-ups on both lines
    let i2c = I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        p.DMA1_CH7,
        p.DMA1_CH2,
        Irqs,
        i2c::Config::default(),
    );
    // sensor's active-low reset (PB8): drive high to keep it running
    let sensor_n_reset = Output::new(p.PB8, Level::High, Speed::Low);
    spawner.spawn(temperature_task(i2c, sensor_n_reset).unwrap());
}

#[panic_handler]
fn hard_fault_handler(_info: &PanicInfo) -> ! {
    SCB::sys_reset(); // reset STM
}
