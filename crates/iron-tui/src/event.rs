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
