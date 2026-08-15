use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "wsentry",
    version,
    about = "Terminal-native system and service monitoring"
)]
pub struct Cli {
    /// Project directory to monitor. Defaults to the current machine.
    #[arg(value_name = "PATH")]
    pub target: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the TUI with deterministic simulated data.
    Demo,
    /// Create a starter wsentry.toml in a project directory.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Open the TUI directly on the process view.
    Processes,
    /// Open the TUI directly on configured project logs.
    Logs,
    /// Open the TUI directly on listening ports and active sockets.
    Ports,
    /// Run configured service checks once and exit.
    Check {
        #[arg(short, long)]
        config: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Print a point-in-time system diagnostic report.
    Report {
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Remove host names, command lines, addresses, and service targets.
        #[arg(long)]
        redact: bool,
    },
    /// Validate a wsentry.toml without starting the TUI.
    Validate {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Inspect terminal, platform, configuration, and collector support.
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_redacted_json_report() {
        let cli = Cli::try_parse_from(["wsentry", "report", "--format", "json", "--redact"])
            .expect("report command parses");
        assert!(matches!(
            cli.command,
            Some(Command::Report {
                format: OutputFormat::Json,
                redact: true,
                ..
            })
        ));
    }

    #[test]
    fn parses_validate_with_explicit_config() {
        let cli = Cli::try_parse_from(["wsentry", "validate", "--config", "project.toml"])
            .expect("validate command parses");
        assert!(matches!(
            cli.command,
            Some(Command::Validate { config: Some(path) })
                if path.as_path() == std::path::Path::new("project.toml")
        ));
    }
}
