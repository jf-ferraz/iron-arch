//! Event handling for Iron TUI
//!
//! Provides async event loop with keyboard, mouse, and tick events.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// TUI events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Periodic tick for updates
    Tick,
}

/// Event handler that polls for terminal events
pub struct EventHandler {
    /// Event receiver channel
    receiver: mpsc::Receiver<Event>,
    /// Event sender (kept for potential future use)
    _sender: mpsc::Sender<Event>,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let event_sender = sender.clone();

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // Calculate timeout until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                // Poll for events
                if event::poll(timeout).expect("Failed to poll events") {
                    match event::read().expect("Failed to read event") {
                        CrosstermEvent::Key(key) => {
                            if event_sender.send(Event::Key(key)).is_err() {
                                return;
                            }
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            if event_sender.send(Event::Mouse(mouse)).is_err() {
                                return;
                            }
                        }
                        CrosstermEvent::Resize(width, height) => {
                            if event_sender.send(Event::Resize(width, height)).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                // Send tick event if enough time has passed
                if last_tick.elapsed() >= tick_rate {
                    if event_sender.send(Event::Tick).is_err() {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            receiver,
            _sender: sender,
        }
    }

    /// Receive the next event (blocking)
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.receiver.recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

    // ==========================================================================
    // Event Enum Tests
    // ==========================================================================

    #[test]
    fn test_event_key_creation() {
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let event = Event::Key(key_event);

        // Verify it can be matched
        match event {
            Event::Key(k) => assert_eq!(k.code, KeyCode::Char('a')),
            _ => panic!("Expected Key event"),
        }
    }

    #[test]
    fn test_event_key_with_modifiers() {
        let key_event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let event = Event::Key(key_event);

        match event {
            Event::Key(k) => {
                assert_eq!(k.code, KeyCode::Char('c'));
                assert!(k.modifiers.contains(KeyModifiers::CONTROL));
            }
            _ => panic!("Expected Key event"),
        }
    }

    #[test]
    fn test_event_mouse_creation() {
        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 20,
            modifiers: KeyModifiers::empty(),
        };
        let event = Event::Mouse(mouse_event);

        match event {
            Event::Mouse(m) => {
                assert_eq!(m.column, 10);
                assert_eq!(m.row, 20);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_event_resize_creation() {
        let event = Event::Resize(80, 24);

        match event {
            Event::Resize(w, h) => {
                assert_eq!(w, 80);
                assert_eq!(h, 24);
            }
            _ => panic!("Expected Resize event"),
        }
    }

    #[test]
    fn test_event_tick_creation() {
        let event = Event::Tick;

        match event {
            Event::Tick => {} // Success
            _ => panic!("Expected Tick event"),
        }
    }

    // ==========================================================================
    // Event Clone Tests
    // ==========================================================================

    #[test]
    fn test_event_key_clone() {
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let event = Event::Key(key_event);
        let cloned = event.clone();

        match cloned {
            Event::Key(k) => assert_eq!(k.code, KeyCode::Enter),
            _ => panic!("Clone should preserve event type"),
        }
    }

    #[test]
    fn test_event_resize_clone() {
        let event = Event::Resize(120, 40);
        let cloned = event.clone();

        match cloned {
            Event::Resize(w, h) => {
                assert_eq!(w, 120);
                assert_eq!(h, 40);
            }
            _ => panic!("Clone should preserve event type"),
        }
    }

    #[test]
    fn test_event_tick_clone() {
        let event = Event::Tick;
        let cloned = event.clone();

        match cloned {
            Event::Tick => {} // Success
            _ => panic!("Clone should preserve event type"),
        }
    }

    // ==========================================================================
    // Event Debug Tests
    // ==========================================================================

    #[test]
    fn test_event_key_debug() {
        let key_event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        let event = Event::Key(key_event);
        let debug_str = format!("{:?}", event);

        assert!(debug_str.contains("Key"));
    }

    #[test]
    fn test_event_mouse_debug() {
        let mouse_event = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let event = Event::Mouse(mouse_event);
        let debug_str = format!("{:?}", event);

        assert!(debug_str.contains("Mouse"));
    }

    #[test]
    fn test_event_resize_debug() {
        let event = Event::Resize(100, 50);
        let debug_str = format!("{:?}", event);

        assert!(debug_str.contains("Resize"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("50"));
    }

    #[test]
    fn test_event_tick_debug() {
        let event = Event::Tick;
        let debug_str = format!("{:?}", event);

        assert!(debug_str.contains("Tick"));
    }

    // ==========================================================================
    // Event Pattern Matching Tests
    // ==========================================================================

    #[test]
    fn test_event_pattern_match_all_variants() {
        let events = vec![
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::empty(),
            }),
            Event::Resize(80, 24),
            Event::Tick,
        ];

        let mut counts = [0usize; 4];
        for event in events {
            match event {
                Event::Key(_) => counts[0] += 1,
                Event::Mouse(_) => counts[1] += 1,
                Event::Resize(_, _) => counts[2] += 1,
                Event::Tick => counts[3] += 1,
            }
        }

        assert_eq!(counts, [1, 1, 1, 1]);
    }

    // ==========================================================================
    // Special Key Events Tests
    // ==========================================================================

    #[test]
    fn test_event_special_keys() {
        let special_keys = vec![
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Esc,
            KeyCode::F(1),
            KeyCode::F(12),
        ];

        for code in special_keys {
            let event = Event::Key(KeyEvent::new(code, KeyModifiers::empty()));
            match event {
                Event::Key(k) => assert_eq!(k.code, code),
                _ => panic!("Should be Key event"),
            }
        }
    }

    #[test]
    fn test_event_modifier_combinations() {
        let modifiers = vec![
            KeyModifiers::CONTROL,
            KeyModifiers::SHIFT,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ];

        for mods in modifiers {
            let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), mods));
            match event {
                Event::Key(k) => assert_eq!(k.modifiers, mods),
                _ => panic!("Should be Key event"),
            }
        }
    }
}
