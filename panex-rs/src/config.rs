#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Exited(i32),
    Failed(String),
    Stopped,
}

impl ProcessStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            ProcessStatus::Running => "●",
            ProcessStatus::Exited(0) => "○",
            ProcessStatus::Exited(_) => "✗",
            ProcessStatus::Failed(_) => "✗",
            ProcessStatus::Stopped => "○",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            ProcessStatus::Running => Color::Green,
            ProcessStatus::Exited(0) => Color::Gray,
            ProcessStatus::Exited(_) => Color::Red,
            ProcessStatus::Failed(_) => Color::Red,
            ProcessStatus::Stopped => Color::Gray,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub name: String,
    pub command: String,
    pub no_shift_tab: bool, // Per-process shift-tab disable
    pub wrap_enabled: bool, // Per-process line wrapping
}

#[derive(Debug, Clone)]
pub struct PanexConfig {
    pub processes: Vec<ProcessConfig>,
    pub no_shift_tab: bool,
    pub timeout: u64,
}

impl PanexConfig {
    pub fn from_args(commands: Vec<String>, names: Option<String>, no_shift_tab: bool, timeout: u64) -> Self {
        let name_list: Vec<String> = names
            .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let processes = commands
            .into_iter()
            .enumerate()
            .map(|(i, cmd)| {
                let raw_name = name_list
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("proc{}", i + 1));
                // Parse suffixes: '!' disables shift-tab, ':w' enables wrapping
                // Suffixes set flags but name keeps original form (for uniqueness)
                let name = raw_name.clone();
                let mut temp = raw_name;
                let mut proc_no_shift_tab = false;
                let mut wrap_enabled = false;

                // Loop to detect suffixes in any order
                loop {
                    if temp.ends_with('!') {
                        temp = temp.trim_end_matches('!').to_string();
                        proc_no_shift_tab = true;
                    } else if temp.ends_with(":w") {
                        temp = temp.trim_end_matches(":w").to_string();
                        wrap_enabled = true;
                    } else {
                        break;
                    }
                }
                ProcessConfig {
                    name,
                    command: cmd,
                    no_shift_tab: proc_no_shift_tab,
                    wrap_enabled,
                }
            })
            .collect();

        PanexConfig {
            processes,
            no_shift_tab,
            timeout,
        }
    }
}
