//! Secret easter egg sequence detection logic.
//!
//! Pure detector without GTK types or dependencies.
//! Detects the Konami code sequence: Up Up Down Down Left Right Left Right B A.

use std::time::{Duration, Instant};

/// Abstract key representation for sequence detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorKey {
    Up,
    Down,
    Left,
    Right,
    Char(char),
    Modifier,
    Other,
}

impl From<char> for DetectorKey {
    fn from(c: char) -> Self {
        DetectorKey::Char(c)
    }
}

/// The target Konami code sequence.
pub const KONAMI_SEQUENCE: [DetectorKey; 10] = [
    DetectorKey::Up,
    DetectorKey::Up,
    DetectorKey::Down,
    DetectorKey::Down,
    DetectorKey::Left,
    DetectorKey::Right,
    DetectorKey::Left,
    DetectorKey::Right,
    DetectorKey::Char('b'),
    DetectorKey::Char('a'),
];

/// Timeout duration after which sequence progress resets (3 seconds).
pub const SEQUENCE_TIMEOUT: Duration = Duration::from_secs(3);

/// Pure sequence detector with clock injection for testability.
pub struct SequenceDetector<F = fn() -> Instant> {
    progress: usize,
    last_press: Option<Instant>,
    completed: bool,
    clock: F,
}

impl SequenceDetector<fn() -> Instant> {
    /// Create a new sequence detector using `Instant::now` as the clock.
    pub fn new() -> Self {
        Self::with_clock(Instant::now)
    }
}

impl Default for SequenceDetector<fn() -> Instant> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Fn() -> Instant> SequenceDetector<F> {
    /// Create a new sequence detector with an injected clock function.
    pub fn with_clock(clock: F) -> Self {
        Self {
            progress: 0,
            last_press: None,
            completed: false,
            clock,
        }
    }

    /// Feed a key press using the injected clock.
    /// Returns true exactly once when the sequence completes.
    pub fn feed(&mut self, key: DetectorKey) -> bool {
        let now = (self.clock)();
        self.feed_at(key, now)
    }

    /// Feed a key press at an explicit instant.
    /// Returns true exactly once when the sequence completes.
    pub fn feed_at(&mut self, key: DetectorKey, now: Instant) -> bool {
        if self.completed {
            return false;
        }

        // Modifier-only presses are ignored completely.
        if key == DetectorKey::Modifier {
            return false;
        }

        // Check timeout: progress resets after 3 seconds without a key.
        if let Some(last) = self.last_press {
            if now.saturating_duration_since(last) >= SEQUENCE_TIMEOUT {
                self.progress = 0;
            }
        }

        let expected = KONAMI_SEQUENCE[self.progress];
        if keys_match(key, expected) {
            self.progress += 1;
            self.last_press = Some(now);
            if self.progress == KONAMI_SEQUENCE.len() {
                self.completed = true;
                return true;
            }
            false
        } else {
            self.last_press = Some(now);
            // If the wrong key equals the first key, restart at progress 1.
            if keys_match(key, KONAMI_SEQUENCE[0]) {
                self.progress = 1;
            } else {
                self.progress = 0;
            }
            false
        }
    }

    /// Current progress count (number of matched keys so far).
    pub fn progress(&self) -> usize {
        self.progress
    }

    /// Whether the full sequence has already completed.
    pub fn is_completed(&self) -> bool {
        self.completed
    }
}

/// Case-insensitive key comparison helper.
fn keys_match(actual: DetectorKey, expected: DetectorKey) -> bool {
    match (actual, expected) {
        (DetectorKey::Up, DetectorKey::Up) => true,
        (DetectorKey::Down, DetectorKey::Down) => true,
        (DetectorKey::Left, DetectorKey::Left) => true,
        (DetectorKey::Right, DetectorKey::Right) => true,
        (DetectorKey::Char(a), DetectorKey::Char(b)) => a.eq_ignore_ascii_case(&b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    type MockClock = Box<dyn Fn() -> Instant>;
    type TestSetup = (SequenceDetector<MockClock>, Rc<Cell<Instant>>);

    fn make_test_detector() -> TestSetup {
        let now = Instant::now();
        let time_cell = Rc::new(Cell::new(now));
        let time_clone = Rc::clone(&time_cell);
        let clock: Box<dyn Fn() -> Instant> = Box::new(move || time_clone.get());
        let detector = SequenceDetector::with_clock(clock);
        (detector, time_cell)
    }

    #[test]
    fn test_success() {
        let (mut detector, _) = make_test_detector();
        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Down));
        assert!(!detector.feed(DetectorKey::Down));
        assert!(!detector.feed(DetectorKey::Left));
        assert!(!detector.feed(DetectorKey::Right));
        assert!(!detector.feed(DetectorKey::Left));
        assert!(!detector.feed(DetectorKey::Right));
        assert!(!detector.feed(DetectorKey::Char('b')));
        assert!(detector.feed(DetectorKey::Char('a')));
        assert!(detector.is_completed());
    }

    #[test]
    fn test_wrong_key() {
        let (mut detector, _) = make_test_detector();
        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Up));
        assert_eq!(detector.progress(), 2);

        // Wrong key: 'x' resets progress to 0
        assert!(!detector.feed(DetectorKey::Char('x')));
        assert_eq!(detector.progress(), 0);
    }

    #[test]
    fn test_restart_on_first_key() {
        let (mut detector, _) = make_test_detector();
        // Up Up Up Down Down Left Right Left Right B A
        assert!(!detector.feed(DetectorKey::Up)); // 1
        assert!(!detector.feed(DetectorKey::Up)); // 2
        assert!(!detector.feed(DetectorKey::Up)); // wrong key for step 3, but equals first key (Up) -> restarts at 1
        assert_eq!(detector.progress(), 1);
        assert!(!detector.feed(DetectorKey::Up)); // 2
        assert!(!detector.feed(DetectorKey::Down)); // 3
        assert!(!detector.feed(DetectorKey::Down)); // 4
        assert!(!detector.feed(DetectorKey::Left)); // 5
        assert!(!detector.feed(DetectorKey::Right)); // 6
        assert!(!detector.feed(DetectorKey::Left)); // 7
        assert!(!detector.feed(DetectorKey::Right)); // 8
        assert!(!detector.feed(DetectorKey::Char('b'))); // 9
        assert!(detector.feed(DetectorKey::Char('a'))); // 10
        assert!(detector.is_completed());
    }

    #[test]
    fn test_timeout() {
        let (mut detector, time_cell) = make_test_detector();
        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Up));
        assert_eq!(detector.progress(), 2);

        // Advance simulated time by 3.1 seconds
        time_cell.set(time_cell.get() + Duration::from_millis(3100));

        // After timeout, non-matching key leaves progress at 0
        assert!(!detector.feed(DetectorKey::Down));
        assert_eq!(detector.progress(), 0);

        // But after timeout, pressing Up starts fresh at 1
        assert!(!detector.feed(DetectorKey::Up));
        assert_eq!(detector.progress(), 1);
    }

    #[test]
    fn test_trigger_fires_once() {
        let (mut detector, _) = make_test_detector();
        for &key in &KONAMI_SEQUENCE[..9] {
            assert!(!detector.feed(key));
        }
        assert!(detector.feed(KONAMI_SEQUENCE[9]));

        // Subsequent keys, including a full sequence replay, return false
        for &key in &KONAMI_SEQUENCE {
            assert!(!detector.feed(key));
        }
        assert!(detector.is_completed());
    }

    #[test]
    fn test_ignores_modifiers() {
        let (mut detector, _) = make_test_detector();
        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert_eq!(detector.progress(), 1);

        assert!(!detector.feed(DetectorKey::Up));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert_eq!(detector.progress(), 2);

        assert!(!detector.feed(DetectorKey::Down));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert!(!detector.feed(DetectorKey::Down));
        assert!(!detector.feed(DetectorKey::Left));
        assert!(!detector.feed(DetectorKey::Right));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert!(!detector.feed(DetectorKey::Left));
        assert!(!detector.feed(DetectorKey::Right));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert!(!detector.feed(DetectorKey::Char('b')));
        assert!(!detector.feed(DetectorKey::Modifier));
        assert!(detector.feed(DetectorKey::Char('a')));
        assert!(detector.is_completed());
    }

    #[test]
    fn test_case_insensitive_letters() {
        let (mut detector, _) = make_test_detector();
        for &key in &KONAMI_SEQUENCE[..8] {
            assert!(!detector.feed(key));
        }
        // Use uppercase 'B' and 'A'
        assert!(!detector.feed(DetectorKey::Char('B')));
        assert!(detector.feed(DetectorKey::Char('A')));
        assert!(detector.is_completed());
    }
}
