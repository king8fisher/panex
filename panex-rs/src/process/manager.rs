use super::{PtyHandle, TerminalBuffer};
use crate::config::{ProcessConfig, ProcessStatus};
use crate::event::{AppEvent, Generation};
use anyhow::Result;
use chrono::Local;
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ManagedProcess {
    pub config: ProcessConfig,
    pub status: ProcessStatus,
    pub buffer: TerminalBuffer,
    pub pty: Option<PtyHandle>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub wrap_enabled: bool,
    pub generation: Generation,
    shutdown: Arc<AtomicBool>,
}

impl ManagedProcess {
    pub fn new(config: ProcessConfig, cols: usize, rows: usize, max_scrollback: usize) -> Self {
        let wrap_enabled = config.wrap_enabled;
        Self {
            config,
            status: ProcessStatus::Stopped,
            buffer: TerminalBuffer::with_max_scrollback(cols, rows, max_scrollback),
            pty: None,
            scroll_offset: 0,
            auto_scroll: true,
            wrap_enabled,
            generation: 0,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct ProcessManager {
    processes: HashMap<String, ManagedProcess>,
    process_order: Vec<String>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    cols: u16,
    rows: u16,
    timeout: u64,
    buffer_size: usize,
    show_restart_marker: bool,
}

impl ProcessManager {
    pub fn new(
        event_tx: mpsc::UnboundedSender<AppEvent>,
        cols: u16,
        rows: u16,
        timeout: u64,
        buffer_size: usize,
        show_restart_marker: bool,
    ) -> Self {
        Self {
            processes: HashMap::new(),
            process_order: Vec::new(),
            event_tx,
            cols,
            rows,
            timeout,
            buffer_size,
            show_restart_marker,
        }
    }

    pub fn add_process(&mut self, config: ProcessConfig) {
        let name = config.name.clone();
        let process = ManagedProcess::new(
            config,
            self.cols as usize,
            self.rows as usize,
            self.buffer_size,
        );
        self.processes.insert(name.clone(), process);
        self.process_order.push(name);
    }

    pub fn start_process(&mut self, name: &str) -> Result<()> {
        let process = self
            .processes
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process not found: {}", name))?;

        // Stop existing reader thread
        process.shutdown.store(true, Ordering::SeqCst);

        // Increment generation so old events are ignored
        process.generation += 1;
        let generation = process.generation;

        let pty = PtyHandle::spawn(&process.config.command, self.cols, self.rows)?;
        process.pty = Some(pty);
        process.status = ProcessStatus::Running;
        process.shutdown = Arc::new(AtomicBool::new(false));

        // Spawn reader thread
        let reader = process.pty.as_ref().unwrap().get_reader();
        let tx = self.event_tx.clone();
        let proc_name = name.to_string();
        let shutdown = Arc::clone(&process.shutdown);

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                let mut reader_guard = match reader.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };

                match reader_guard.read(&mut buf) {
                    Ok(0) => {
                        drop(reader_guard);
                        let _ =
                            tx.send(AppEvent::ProcessExited(proc_name.clone(), generation, None));
                        break;
                    }
                    Ok(n) => {
                        drop(reader_guard);
                        let _ = tx.send(AppEvent::ProcessOutput(
                            proc_name.clone(),
                            generation,
                            buf[..n].to_vec(),
                        ));
                    }
                    Err(e) => {
                        drop(reader_guard);
                        if !shutdown.load(Ordering::SeqCst) {
                            let _ = tx.send(AppEvent::ProcessError(
                                proc_name.clone(),
                                generation,
                                e.to_string(),
                            ));
                        }
                        break;
                    }
                }
            }
        });

        let _ = self
            .event_tx
            .send(AppEvent::ProcessStarted(name.to_string()));
        Ok(())
    }

    pub fn start_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.process_order.clone();
        for name in names {
            self.start_process(&name)?;
        }
        Ok(())
    }

    pub fn restart_process(&mut self, name: &str) -> Result<()> {
        let timestamp = self.show_restart_marker.then(Self::restart_timestamp);
        self.kill_process(name)?;
        // Generation counter ensures old events are ignored, minimal delay needed
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.apply_restart_output_action(name, timestamp.as_deref())?;
        self.start_process(name)
    }

    pub fn restart_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.process_order.clone();
        let timestamp = self.show_restart_marker.then(Self::restart_timestamp);
        // Kill all first
        for name in &names {
            let _ = self.kill_process(name);
        }
        // Brief delay for cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));
        for name in &names {
            self.apply_restart_output_action(name, timestamp.as_deref())?;
        }
        // Start all - generation counter ensures old events are ignored
        for name in names {
            self.start_process(&name)?;
        }
        Ok(())
    }

    fn restart_timestamp() -> String {
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn apply_restart_output_action(&mut self, name: &str, timestamp: Option<&str>) -> Result<()> {
        if self.show_restart_marker {
            self.append_restart_marker(
                name,
                timestamp
                    .expect("restart marker timestamp must be set when marker mode is enabled"),
            )
        } else {
            self.clear_restart_output(name)
        }
    }

    fn clear_restart_output(&mut self, name: &str) -> Result<()> {
        let process = self
            .processes
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process not found: {}", name))?;
        process.buffer.clear_for_restart();
        process.scroll_offset = 0;
        process.auto_scroll = true;
        Ok(())
    }

    fn append_restart_marker(&mut self, name: &str, timestamp: &str) -> Result<()> {
        let process = self
            .processes
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process not found: {}", name))?;
        let marker = Self::restart_marker(timestamp);
        process.buffer.write(marker.as_bytes());
        Ok(())
    }

    fn restart_marker(timestamp: &str) -> String {
        const BLUE: &str = "\x1b[34m";
        const RESET: &str = "\x1b[0m";

        let text = format!("Restarted {timestamp}");
        let padded = format!("  {text}  ");
        let border = "─".repeat(padded.chars().count());

        format!(
            "\r\n{BLUE}┌{border}┐{RESET}\r\n{BLUE}│{RESET}{padded}{BLUE}│{RESET}\r\n{BLUE}└{border}┘{RESET}\r\n"
        )
    }

    pub fn kill_process(&mut self, name: &str) -> Result<()> {
        if let Some(process) = self.processes.get_mut(name) {
            process.shutdown.store(true, Ordering::SeqCst);
            if let Some(ref pty) = process.pty {
                let _ = pty.kill(self.timeout);
            }
            process.pty = None;
            process.status = ProcessStatus::Stopped;
        }
        Ok(())
    }

    pub fn write_to_process(&self, name: &str, data: &[u8]) -> Result<()> {
        if let Some(process) = self.processes.get(name) {
            if let Some(ref pty) = process.pty {
                pty.write(data)?;
            }
        }
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        for process in self.processes.values_mut() {
            process.buffer.resize(cols as usize, rows as usize);
            if let Some(ref pty) = process.pty {
                let _ = pty.resize(cols, rows);
            }
        }
    }

    /// Re-send PTY resize to a process (triggers SIGWINCH). Used on focus change.
    pub fn nudge_resize(&self, name: &str) {
        if let Some(process) = self.processes.get(name) {
            if let Some(ref pty) = process.pty {
                let _ = pty.resize(self.cols, self.rows);
            }
        }
    }

    pub fn handle_output(&mut self, name: &str, gen: Generation, data: &[u8]) {
        if let Some(process) = self.processes.get_mut(name) {
            // Ignore events from old process instances
            if process.generation != gen {
                return;
            }

            process.buffer.write(data);

            // Send any pending responses (e.g., device attributes queries)
            let responses = process.buffer.take_pending_responses();
            if let Some(ref pty) = process.pty {
                for response in responses {
                    let _ = pty.write(&response);
                }
            }

            if process.auto_scroll {
                let visible = self.rows as usize;
                let lines = process.buffer.get_all_lines();
                // Exclude trailing empty lines (consistent with render)
                let content_count = {
                    let mut count = lines.len();
                    while count > 0 && lines[count - 1].cells.is_empty() {
                        count -= 1;
                    }
                    count.max(1)
                };
                let total_display_lines = if process.wrap_enabled && self.cols > 0 {
                    let cols = self.cols as usize;
                    lines
                        .iter()
                        .take(content_count)
                        .map(|line| {
                            if line.cells.is_empty() {
                                1
                            } else {
                                line.cells.len().div_ceil(cols)
                            }
                        })
                        .sum::<usize>()
                        .max(1)
                } else {
                    content_count
                };
                // Scroll to show bottom of content
                if total_display_lines > visible {
                    process.scroll_offset = total_display_lines - visible;
                } else {
                    process.scroll_offset = 0;
                }
            }
        }
    }

    pub fn handle_exit(&mut self, name: &str, gen: Generation, code: Option<i32>) {
        if let Some(process) = self.processes.get_mut(name) {
            // Ignore events from old process instances
            if process.generation != gen {
                return;
            }

            process.shutdown.store(true, Ordering::SeqCst);
            process.pty = None;
            process.status = match code {
                Some(c) => ProcessStatus::Exited(c),
                None => ProcessStatus::Exited(0),
            };
        }
    }

    pub fn handle_error(&mut self, name: &str, gen: Generation, error: &str) {
        if let Some(process) = self.processes.get_mut(name) {
            // Ignore events from old process instances
            if process.generation != gen {
                return;
            }

            process.shutdown.store(true, Ordering::SeqCst);
            process.pty = None;
            process.status = ProcessStatus::Failed(error.to_string());
        }
    }

    pub fn get_process(&self, name: &str) -> Option<&ManagedProcess> {
        self.processes.get(name)
    }

    pub fn get_process_mut(&mut self, name: &str) -> Option<&mut ManagedProcess> {
        self.processes.get_mut(name)
    }

    pub fn process_names(&self) -> &[String] {
        &self.process_order
    }

    pub fn process_count(&self) -> usize {
        self.process_order.len()
    }

    /// Begin graceful shutdown - send SIGTERM to all running processes
    pub fn begin_shutdown(&mut self) {
        for name in self.process_order.clone() {
            if let Some(process) = self.processes.get_mut(&name) {
                if matches!(process.status, ProcessStatus::Running) {
                    process.shutdown.store(true, Ordering::SeqCst);
                    if let Some(ref pty) = process.pty {
                        let _ = pty.terminate();
                    }
                }
            }
        }
    }

    /// Count processes that are not running
    pub fn stopped_count(&self) -> usize {
        self.processes
            .values()
            .filter(|p| !matches!(p.status, ProcessStatus::Running))
            .count()
    }

    /// Check if any process is still running
    pub fn any_running(&self) -> bool {
        self.processes
            .values()
            .any(|p| matches!(p.status, ProcessStatus::Running))
    }

    /// Force kill all remaining processes
    pub fn finish_shutdown(&mut self) {
        for name in self.process_order.clone() {
            if let Some(process) = self.processes.get_mut(&name) {
                if matches!(process.status, ProcessStatus::Running) {
                    if let Some(ref pty) = process.pty {
                        let _ = pty.force_kill();
                    }
                    process.pty = None;
                    process.status = ProcessStatus::Stopped;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use tokio::sync::mpsc;

    fn process_config(name: &str) -> ProcessConfig {
        ProcessConfig {
            name: name.to_string(),
            command: "true".to_string(),
            no_shift_tab: false,
            wrap_enabled: false,
        }
    }

    fn test_manager(names: &[&str]) -> ProcessManager {
        test_manager_with_restart_marker(names, false)
    }

    fn test_manager_with_restart_marker(
        names: &[&str],
        show_restart_marker: bool,
    ) -> ProcessManager {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let mut pm = ProcessManager::new(event_tx, 80, 24, 500, 10_000, show_restart_marker);

        for name in names {
            pm.add_process(process_config(name));
        }

        pm
    }

    fn expected_restart_box(timestamp: &str) -> String {
        let text = format!("Restarted {timestamp}");
        let padded = format!("  {text}  ");
        let border = "─".repeat(padded.chars().count());
        format!("┌{border}┐\n│{padded}│\n└{border}┘")
    }

    #[test]
    fn append_restart_marker_starts_on_separate_line_after_partial_output() {
        let mut pm = test_manager(&["one"]);
        pm.get_process_mut("one")
            .unwrap()
            .buffer
            .write(b"old output");

        pm.append_restart_marker("one", "2026-05-08 12:34:56")
            .unwrap();

        let output = pm.get_process("one").unwrap().buffer.to_test_string();
        assert_eq!(
            output,
            format!(
                "old output\n{}",
                expected_restart_box("2026-05-08 12:34:56")
            )
        );
    }

    #[test]
    fn restart_marker_uses_blue_box_border() {
        let mut pm = test_manager(&["one"]);

        pm.append_restart_marker("one", "2026-05-08 12:34:56")
            .unwrap();

        let lines = pm.get_process("one").unwrap().buffer.get_all_lines();
        assert!(lines[1]
            .cells
            .iter()
            .all(|cell| cell.style.fg == Some(Color::Blue)));
        assert_eq!(lines[2].cells[0].style.fg, Some(Color::Blue));
        assert_eq!(lines[2].cells.last().unwrap().style.fg, Some(Color::Blue));
        assert_eq!(lines[2].cells[1].style.fg, None);
        assert_eq!(lines[3].cells[0].style.fg, Some(Color::Blue));
    }

    #[test]
    fn output_written_after_restart_marker_appears_on_next_line() {
        let mut pm = test_manager(&["one"]);
        pm.get_process_mut("one")
            .unwrap()
            .buffer
            .write(b"old output");

        pm.append_restart_marker("one", "2026-05-08 12:34:56")
            .unwrap();
        pm.get_process_mut("one")
            .unwrap()
            .buffer
            .write(b"new output");

        let output = pm.get_process("one").unwrap().buffer.to_test_string();
        assert_eq!(
            output,
            format!(
                "old output\n{}\nnew output",
                expected_restart_box("2026-05-08 12:34:56")
            )
        );
    }

    #[test]
    fn restart_process_clears_old_output_by_default_without_marker() {
        let mut pm = test_manager(&["one"]);
        pm.get_process_mut("one")
            .unwrap()
            .buffer
            .write(b"old output");

        pm.restart_process("one").unwrap();

        let output = pm.get_process("one").unwrap().buffer.to_test_string();
        assert_eq!(output, "");
        assert!(!output.contains("Restarted"));
    }

    #[test]
    fn restart_all_clears_every_process_by_default() {
        let mut pm = test_manager(&["one", "two"]);
        for name in ["one", "two"] {
            pm.get_process_mut(name)
                .unwrap()
                .buffer
                .write(b"old output");
        }

        pm.restart_all().unwrap();

        let one = pm.get_process("one").unwrap().buffer.to_test_string();
        let two = pm.get_process("two").unwrap().buffer.to_test_string();

        assert_eq!(one, "");
        assert_eq!(two, "");
    }

    #[test]
    fn restart_with_marker_keeps_old_output_and_appends_marker() {
        let mut pm = test_manager_with_restart_marker(&["one"], true);
        pm.get_process_mut("one")
            .unwrap()
            .buffer
            .write(b"old output");

        pm.restart_process("one").unwrap();

        let output = pm.get_process("one").unwrap().buffer.to_test_string();
        assert!(output.starts_with("old output\n┌"));
        assert!(output.contains("\n│  Restarted "));
        assert!(output.ends_with("┘"));
    }

    #[test]
    fn restart_all_appends_same_marker_to_every_process_when_enabled() {
        let mut pm = test_manager_with_restart_marker(&["one", "two"], true);
        for name in ["one", "two"] {
            pm.get_process_mut(name)
                .unwrap()
                .buffer
                .write(b"old output");
        }

        pm.restart_all().unwrap();

        let one = pm.get_process("one").unwrap().buffer.to_test_string();
        let two = pm.get_process("two").unwrap().buffer.to_test_string();

        assert!(one.starts_with("old output\n┌"));
        assert!(one.contains("\n│  Restarted "));
        assert!(one.ends_with("┘"));
        assert_eq!(one, two);
    }
}
