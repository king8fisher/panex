use crossterm::event::Event as CrosstermEvent;

/// Generation counter to distinguish events from old vs new process instances
pub type Generation = u64;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    Input(CrosstermEvent),
    ProcessOutput(String, Generation, Vec<u8>),
    ProcessStarted(String),
    ProcessExited(String, Generation, Option<i32>),
    ProcessError(String, Generation, String),
    Tick,
}
