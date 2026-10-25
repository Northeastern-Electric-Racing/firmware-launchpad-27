//! Pure input qualification; desktop tests do not require an STM32.

#[derive(Clone, Copy)]
pub enum ButtonType {
    Direction,
    Accelerate,
    Brake,
}

#[derive(Clone, Copy)]
pub struct ButtonPress {
    pub kind: ButtonType,
    pub started_ms: u64,
}

impl ButtonPress {
    /// Reject presses begun before/during fault recovery, even if delivered late.
    pub fn is_fresh(self, ready_since_ms: u64) -> bool {
        self.started_ms > ready_since_ms
    }
}

pub struct Debouncer {
    candidate: bool,
    stable: bool,
    since_ms: u64,
    pending: bool,
    armed: bool,
}

impl Debouncer {
    pub fn new(pressed: bool, now_ms: u64) -> Self {
        Self {
            candidate: pressed,
            stable: pressed,
            since_ms: now_ms,
            pending: true,
            // Require a stable release first, including after boot while held.
            armed: false,
        }
    }

    pub fn deadline_ms(&self) -> Option<u64> {
        self.pending.then_some(self.since_ms + 30)
    }
    pub fn stable_pressed(&self) -> bool {
        self.stable
    }

    /// Every observed edge restarts qualification, even if several transitions
    /// leave the sampled level unchanged. Return the accepted press's start time.
    pub fn update(&mut self, pressed: bool, now_ms: u64, edge_seen: bool) -> Option<u64> {
        if edge_seen || pressed != self.candidate {
            self.candidate = pressed;
            self.since_ms = now_ms;
            self.pending = true;
            return None;
        }
        if !self.pending || now_ms.saturating_sub(self.since_ms) < 30 {
            return None;
        }
        self.pending = false;
        let changed = self.stable != pressed;
        self.stable = pressed;
        if !pressed {
            self.armed = true;
        } else if changed && self.armed {
            self.armed = false;
            return Some(self.since_ms);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn released() -> Debouncer {
        let mut button = Debouncer::new(false, 0);
        assert_eq!(button.update(false, 30, false), None);
        assert_eq!(button.deadline_ms(), None);
        button
    }

    #[test]
    fn one_press_hold_release_produces_one_event() {
        let mut button = released();
        assert_eq!(button.update(true, 100, true), None);
        assert_eq!(button.update(true, 129, false), None);
        assert_eq!(button.update(true, 130, false), Some(100));
        assert_eq!(button.update(true, 5000, false), None);
        assert_eq!(button.update(false, 5001, true), None);
        assert_eq!(button.update(false, 5031, false), None);
        assert_eq!(button.update(true, 6000, true), None);
        assert_eq!(button.update(true, 6030, false), Some(6000));
    }

    #[test]
    fn bounce_restarts_the_entire_stability_window() {
        let mut button = released();
        button.update(true, 100, true);
        button.update(false, 120, true);
        button.update(true, 125, true);
        assert_eq!(button.update(true, 130, false), None);
        assert_eq!(button.deadline_ms(), Some(155));
        assert_eq!(button.update(true, 155, false), Some(125));
    }

    #[test]
    fn an_observed_edge_resets_even_when_sampled_level_is_unchanged() {
        let mut button = released();
        button.update(true, 100, true);
        button.update(true, 125, true);
        assert_eq!(button.update(true, 130, false), None);
        assert_eq!(button.update(true, 155, false), Some(125));
    }

    #[test]
    fn held_at_boot_requires_release_before_any_press() {
        let mut button = Debouncer::new(true, 0);
        assert_eq!(button.update(true, 30, false), None);
        button.update(false, 40, true);
        button.update(false, 70, false);
        button.update(true, 80, true);
        assert_eq!(button.update(true, 110, false), Some(80));
    }

    #[test]
    fn release_bounce_cannot_rearm_and_duplicate_a_held_press() {
        let mut button = released();
        button.update(true, 100, true);
        assert_eq!(button.update(true, 130, false), Some(100));
        button.update(false, 200, true);
        button.update(true, 225, true);
        assert_eq!(button.update(true, 255, false), None);
        assert_eq!(button.deadline_ms(), None);
    }

    #[test]
    fn delayed_fault_time_events_are_not_fresh_after_recovery() {
        for kind in [
            ButtonType::Accelerate,
            ButtonType::Brake,
            ButtonType::Direction,
        ] {
            assert!(
                !ButtonPress {
                    kind,
                    started_ms: 100
                }
                .is_fresh(150)
            );
            assert!(
                !ButtonPress {
                    kind,
                    started_ms: 150
                }
                .is_fresh(150)
            );
            assert!(
                ButtonPress {
                    kind,
                    started_ms: 151
                }
                .is_fresh(150)
            );
        }
    }
}
