mod config;
mod event;
mod input;
mod process;
mod ui;

use anyhow::Result;
use clap::Parser;
use config::PanexConfig;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::AppEvent;
use futures::StreamExt;
use process::ProcessManager;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    Terminal,
};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);
use ui::{
    help_popup::HelpPopup,
    output_panel::OutputPanel,
    process_list::ProcessList,
    status_bar::StatusBar,
    App,
};

#[derive(Parser, Debug)]
#[command(name = "panex")]
#[command(about = "Process manager with TUI")]
#[command(version)]
struct Cli {
    /// Commands to run
    #[arg(required = true)]
    commands: Vec<String>,

    /// Process names (comma-separated)
    #[arg(short, long)]
    names: Option<String>,

    /// Disable Shift-Tab to exit focus mode
    #[arg(long)]
    no_shift_tab: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.commands.is_empty() {
        eprintln!("Error: At least one command is required");
        std::process::exit(1);
    }

    let config = PanexConfig::from_args(cli.commands, cli.names, cli.no_shift_tab);

    run(config).await
}

async fn run(config: PanexConfig) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config).await;

    // Disable mouse capture first to stop new mouse events
    execute!(terminal.backend_mut(), DisableMouseCapture)?;

    // Drain any pending input events to prevent leakage
    while crossterm::event::poll(std::time::Duration::from_millis(10))? {
        let _ = crossterm::event::read();
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::style::SetAttribute(crossterm::style::Attribute::Reset),
    )?;
    terminal.show_cursor()?;
    io::stdout().flush()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: PanexConfig,
) -> Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();

    let size = terminal.size()?;
    // Output panel width = total - process list (20) - delimiter (1)
    let output_cols = size.width.saturating_sub(21);
    let output_rows = size.height.saturating_sub(1); // -1 for status bar
    let mut pm = ProcessManager::new(event_tx.clone(), output_cols, output_rows);

    // Add processes
    for proc_config in &config.processes {
        pm.add_process(proc_config.clone());
    }

    // Start all processes
    pm.start_all()?;

    let mut app = App::new(config.no_shift_tab);
    let mut event_stream = EventStream::new();
    let mut last_size: Option<(u16, u16)> = None;
    let mut pending_resize: Option<(u16, u16)> = None;
    let mut resize_deadline: Option<Instant> = None;

    loop {
        // Draw
        terminal.draw(|f| {
            let size = f.area();

            let main_chunks = Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(size);

            let content_chunks = Layout::horizontal([
                Constraint::Length(20),
                Constraint::Length(1), // delimiter
                Constraint::Min(0),
            ])
            .split(main_chunks[0]);

            // Process list
            let process_list = ProcessList::new(&pm, app.selected_index, app.mode);
            f.render_widget(process_list, content_chunks[0]);

            // Output panel
            let selected_name = pm.process_names().get(app.selected_index).cloned();
            let selected_process = selected_name.as_ref().and_then(|n| pm.get_process(n));
            let output_panel = OutputPanel::new(selected_process, app.mode);
            f.render_widget(output_panel, content_chunks[2]);

            // Status bar
            let proc_no_shift_tab = selected_process
                .map(|p| p.config.no_shift_tab)
                .unwrap_or(false);
            let status_bar = StatusBar::new(app.mode, app.no_shift_tab, proc_no_shift_tab);
            f.render_widget(status_bar, main_chunks[1]);

            // Help popup
            if app.show_help {
                f.render_widget(HelpPopup::new(), size);
            }
        })?;

        if app.should_quit {
            pm.shutdown();
            // Wait for reader threads to notice shutdown and stop
            tokio::time::sleep(Duration::from_millis(50)).await;
            break;
        }

        let term_size = terminal.size()?;
        let current_size = (term_size.width, term_size.height);

        // Detect size change and schedule debounced resize
        if last_size != Some(current_size) {
            pending_resize = Some(current_size);
            resize_deadline = Some(Instant::now() + RESIZE_DEBOUNCE);
            last_size = Some(current_size);
        }

        // Apply pending resize after debounce period
        if let (Some((cols, rows)), Some(deadline)) = (pending_resize, resize_deadline) {
            if Instant::now() >= deadline {
                pm.resize(cols.saturating_sub(21), rows.saturating_sub(1));
                pending_resize = None;
                resize_deadline = None;
            }
        }

        let visible_height = term_size.height.saturating_sub(1) as usize; // -1 for status bar
        let viewport_width = term_size.width.saturating_sub(21) as usize; // -20 for process list, -1 for delimiter

        // Handle events
        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::ProcessOutput(name, gen, data) => {
                        pm.handle_output(&name, gen, &data);
                    }
                    AppEvent::ProcessStarted(_name) => {
                        // Could show notification
                    }
                    AppEvent::ProcessExited(name, gen, code) => {
                        pm.handle_exit(&name, gen, code);
                    }
                    AppEvent::ProcessError(name, gen, error) => {
                        pm.handle_error(&name, gen, &error);
                    }
                    AppEvent::Input(e) => {
                        if let Some((cols, rows)) = input::handle_event(e, &mut app, &mut pm, visible_height, viewport_width) {
                            // Schedule debounced resize
                            pending_resize = Some((cols, rows));
                            resize_deadline = Some(Instant::now() + RESIZE_DEBOUNCE);
                        }
                    }
                    AppEvent::Tick => {}
                }
            }
            Some(Ok(event)) = event_stream.next() => {
                if let Event::Key(_) | Event::Mouse(_) | Event::Resize(_, _) = event {
                    if let Some((cols, rows)) = input::handle_event(event, &mut app, &mut pm, visible_height, viewport_width) {
                        // Schedule debounced resize
                        pending_resize = Some((cols, rows));
                        resize_deadline = Some(Instant::now() + RESIZE_DEBOUNCE);
                    }
                }
            }
        }
    }

    Ok(())
}
