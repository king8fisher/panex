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
}

#[derive(Debug, Clone)]
pub struct PanexConfig {
    pub processes: Vec<ProcessConfig>,
    pub no_shift_tab: bool,
}

impl PanexConfig {
    pub fn from_args(commands: Vec<String>, names: Option<String>, no_shift_tab: bool) -> Self {
        let name_list: Vec<String> = names
            .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let processes = commands
            .into_iter()
            .enumerate()
            .map(|(i, cmd)| {
                let name = name_list
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("proc{}", i + 1));
                ProcessConfig { name, command: cmd }
            })
            .collect();

        PanexConfig {
            processes,
            no_shift_tab,
        }
    }
}
