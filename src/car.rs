//! Hardware-independent car/fault rules; also compiled directly for desktop tests.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CarDirection {
    Forward,
    Reverse,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Gear {
    Gear1,
    Gear2,
    Gear3,
    Gear4,
}
impl Gear {
    fn from_position_tenths(position: u16) -> Self {
        match position {
            0..250 => Self::Gear1,
            250..500 => Self::Gear2,
            500..750 => Self::Gear3,
            _ => Self::Gear4, // Includes 100.0%.
        }
    }
    fn acceleration(self) -> i32 {
        match self {
            Self::Gear1 => 5,
            Self::Gear2 => 10,
            Self::Gear3 => 20,
            Self::Gear4 => 35,
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct MiniCar {
    velocity: i32,
    gear: Gear,
    direction: CarDirection,
    fault: bool,
}
impl MiniCar {
    pub fn new() -> Self {
        Self {
            velocity: 0,
            gear: Gear::Gear1,
            direction: CarDirection::Forward,
            // Do not drive before the first valid, below-threshold reading.
            fault: true,
        }
    }
    /// Tenths of a percent: 250 = 25.0%, 1000 = 100.0%.
    pub fn set_position(&mut self, position_tenths: u16) {
        self.gear = Gear::from_position_tenths(position_tenths);
    }
    pub fn accelerate(&mut self) {
        if self.fault {
            return;
        }
        let step = self.gear.acceleration();
        let signed_step = match self.direction {
            CarDirection::Forward => step,
            CarDirection::Reverse => -step,
        };
        self.velocity = (self.velocity + signed_step).clamp(-100, 100);
    }
    pub fn brake(&mut self) {
        if self.fault {
            return;
        }
        self.velocity = match self.velocity {
            1.. => (self.velocity - 20).max(0),
            ..=-1 => (self.velocity + 20).min(0),
            _ => 0,
        };
    }
    /// Reverse immediately while preserving absolute speed.
    pub fn reverse(&mut self) {
        if self.fault {
            return;
        }
        self.direction = match self.direction {
            CarDirection::Forward => CarDirection::Reverse,
            CarDirection::Reverse => CarDirection::Forward,
        };
        self.velocity = -self.velocity;
    }
    /// Celsius is already converted by the SHT3x driver. None = read failure.
    pub fn update_temperature(&mut self, celsius: Option<f32>) {
        match celsius {
            Some(value) if value.is_finite() && value < 35.0 => self.fault = false,
            Some(value) if value.is_finite() && value == 35.0 => {}
            _ => self.fault = true,
        }
        if self.fault {
            self.velocity = 0;
        }
    }
    pub fn faulted(&self) -> bool {
        self.fault
    }
    /// None means force Speedometer off; Some(ms) means blink at that interval.
    pub fn led_interval_ms(&self) -> Option<u64> {
        if self.fault {
            None
        } else {
            Some(self.toggle_interval_ms())
        }
    }
    pub fn toggle_interval_ms(&self) -> u64 {
        match self.velocity.unsigned_abs() {
            0..25 => 2500,
            25..50 => 2000,
            50..75 => 1000,
            _ => 500, // Includes exactly 100 m/s, in either direction.
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready_car() -> MiniCar {
        let mut car = MiniCar::new();
        car.update_temperature(Some(25.0));
        car
    }
    #[test]
    fn startup_blocks_motion_until_a_valid_cool_reading() {
        let mut car = MiniCar::new();
        car.accelerate();
        assert!(car.faulted());
        assert_eq!(car.velocity, 0);
        assert_eq!(car.led_interval_ms(), None);
        car.update_temperature(Some(35.0));
        assert!(car.faulted());
        car.update_temperature(Some(34.99));
        assert!(!car.faulted());
        assert_eq!(car.velocity, 0);
        assert_eq!(car.led_interval_ms(), Some(2500));
    }
    #[test]
    fn hot_reading_stops_both_directions_and_locks_all_motion_inputs() {
        for reverse in [false, true] {
            let mut car = ready_car();
            car.set_position(1000);
            if reverse {
                car.reverse();
            }
            car.accelerate();
            assert_eq!(car.velocity.abs(), 35);
            car.update_temperature(Some(35.01));
            assert!(car.faulted());
            assert_eq!(car.velocity, 0);
            assert_eq!(car.led_interval_ms(), None);
            let direction = car.direction;
            for _ in 0..20 {
                car.accelerate();
                car.brake();
                car.reverse();
                car.set_position(500);
                assert_eq!(car.velocity, 0);
                assert_eq!(car.direction, direction);
            }
        }
    }
    #[test]
    fn exact_threshold_keeps_prior_fault_state_and_cooling_does_not_restore_speed() {
        let mut car = ready_car();
        car.accelerate();
        car.update_temperature(Some(35.0));
        assert!(!car.faulted());
        assert_eq!(car.velocity, 5);
        car.update_temperature(Some(36.0));
        car.update_temperature(Some(35.0));
        assert!(car.faulted());
        car.update_temperature(Some(34.99));
        assert!(!car.faulted());
        assert_eq!(car.velocity, 0);
        car.accelerate();
        assert_eq!(car.velocity, 5);
    }
    #[test]
    fn read_errors_and_nonfinite_temperatures_fault_until_valid_cooling() {
        for reading in [
            None,
            Some(f32::NAN),
            Some(f32::INFINITY),
            Some(f32::NEG_INFINITY),
        ] {
            let mut car = ready_car();
            car.accelerate();
            car.update_temperature(reading);
            assert!(car.faulted());
            assert_eq!(car.velocity, 0);
            assert_eq!(car.led_interval_ms(), None);
            car.update_temperature(Some(25.0));
            assert!(!car.faulted());
            assert_eq!(car.velocity, 0);
        }
    }
    #[test]
    fn gear_boundaries_include_full_scale() {
        for (position, expected) in [
            (0, Gear::Gear1),
            (249, Gear::Gear1),
            (250, Gear::Gear2),
            (499, Gear::Gear2),
            (500, Gear::Gear3),
            (749, Gear::Gear3),
            (750, Gear::Gear4),
            (999, Gear::Gear4),
            (1000, Gear::Gear4),
        ] {
            let mut car = ready_car();
            car.set_position(position);
            assert_eq!(car.gear, expected, "position {position}");
        }
    }
    #[test]
    fn each_gear_has_required_signed_acceleration() {
        for (position, step) in [(0, 5), (250, 10), (500, 20), (750, 35)] {
            let mut car = ready_car();
            car.set_position(position);
            car.accelerate();
            assert_eq!(car.velocity, step);
            car.reverse();
            assert_eq!(car.velocity, -step);
            car.accelerate();
            assert_eq!(car.velocity, -2 * step);
        }
    }
    #[test]
    fn repeated_drive_caps_both_directions_at_100() {
        for position in [0, 250, 500, 750, 1000] {
            let mut car = ready_car();
            car.set_position(position);
            for _ in 0..30 {
                car.accelerate();
            }
            assert_eq!(car.velocity, 100);
            car.reverse();
            for _ in 0..30 {
                car.accelerate();
            }
            assert_eq!(car.velocity, -100);
        }
    }
    #[test]
    fn braking_toward_zero_never_crosses_zero() {
        for velocity in -100i32..=100 {
            let mut car = ready_car();
            car.velocity = velocity;
            car.brake();
            assert_eq!(car.velocity.abs(), (velocity.abs() - 20).max(0));
            assert!(car.velocity == 0 || car.velocity.signum() == velocity.signum());
        }
    }
    #[test]
    fn reverse_preserves_speed_and_is_reversible() {
        for velocity in -100..=100 {
            let mut car = ready_car();
            car.velocity = velocity;
            let original = car;
            car.reverse();
            assert_eq!(car.velocity, -velocity);
            assert_eq!(car.direction, CarDirection::Reverse);
            car.reverse();
            assert_eq!(car, original);
        }
    }
    #[test]
    fn blink_bands_use_absolute_speed_and_exact_boundaries() {
        for (speed, interval) in [
            (0, 2500),
            (24, 2500),
            (25, 2000),
            (49, 2000),
            (50, 1000),
            (74, 1000),
            (75, 500),
            (99, 500),
            (100, 500),
        ] {
            for sign in [-1, 1] {
                let mut car = ready_car();
                car.velocity = speed * sign;
                assert_eq!(car.toggle_interval_ms(), interval);
            }
        }
    }
    #[test]
    fn gear_change_requires_drive_to_change_velocity() {
        let mut car = ready_car();
        car.accelerate();
        car.set_position(1000);
        assert_eq!(car.velocity, 5);
        car.accelerate();
        assert_eq!(car.velocity, 40);
        car.brake();
        assert_eq!(car.velocity, 20);
        car.brake();
        car.brake();
        assert_eq!(car.velocity, 0);
    }
    #[test]
    fn mixed_events_preserve_speed_and_direction() {
        let mut car = ready_car();
        for i in 0..10_000 {
            car.set_position((i % 1001) as u16);
            match i % 7 {
                0 => car.reverse(),
                1 | 2 => car.brake(),
                _ => car.accelerate(),
            }
            assert!(car.velocity.abs() <= 100);
            match car.direction {
                CarDirection::Forward => assert!(car.velocity >= 0),
                CarDirection::Reverse => assert!(car.velocity <= 0),
            }
        }
    }
}
