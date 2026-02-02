use super::{PtyHandle, TerminalBuffer};
use crate::config::{ProcessConfig, ProcessStatus};
use crate::event::{AppEvent, Generation};
use anyhow::Result;
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
    pub fn new(config: ProcessConfig, cols: usize, rows: usize) -> Self {
        let wrap_enabled = config.wrap_enabled;
        Self {
            config,
            status: ProcessStatus::Stopped,
            buffer: TerminalBuffer::new(cols, rows),
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
}

impl ProcessManager {
    pub fn new(event_tx: mpsc::UnboundedSender<AppEvent>, cols: u16, rows: u16, timeout: u64) -> Self {
        Self {
            processes: HashMap::new(),
            process_order: Vec::new(),
            event_tx,
            cols,
            rows,
            timeout,
        }
    }

    pub fn add_process(&mut self, config: ProcessConfig) {
        let name = config.name.clone();
        let process = ManagedProcess::new(config, self.cols as usize, self.rows as usize);
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
                        let _ = tx.send(AppEvent::ProcessExited(proc_name.clone(), generation, None));
                        break;
                    }
                    Ok(n) => {
                        drop(reader_guard);
                        let _ = tx.send(AppEvent::ProcessOutput(proc_name.clone(), generation, buf[..n].to_vec()));
                    }
                    Err(e) => {
                        drop(reader_guard);
                        if !shutdown.load(Ordering::SeqCst) {
                            let _ = tx.send(AppEvent::ProcessError(proc_name.clone(), generation, e.to_string()));
                        }
                        break;
                    }
                }
            }
        });

        let _ = self.event_tx.send(AppEvent::ProcessStarted(name.to_string()));
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
        self.kill_process(name)?;
        // Generation counter ensures old events are ignored, minimal delay needed
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.start_process(name)
    }

    pub fn restart_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.process_order.clone();
        // Kill all first
        for name in &names {
            let _ = self.kill_process(name);
        }
        // Brief delay for cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Start all - generation counter ensures old events are ignored
        for name in names {
            self.start_process(&name)?;
        }
        Ok(())
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
                    lines.iter().take(content_count).map(|line| {
                        if line.cells.is_empty() {
                            1
                        } else {
                            (line.cells.len() + cols - 1) / cols
                        }
                    }).sum::<usize>().max(1)
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
