use crossterm::event::Event as CrosstermEvent;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    Input(CrosstermEvent),
    ProcessOutput(String, Vec<u8>),
    ProcessStarted(String),
    ProcessExited(String, Option<i32>),
    ProcessError(String, String),
    Tick,
}
