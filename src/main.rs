mod bluetooth;
mod commands;
mod device;
mod error;
mod output;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use output::{OutputFormat, Report, VersionReport};

#[derive(Parser)]
#[command(
    name = "bose-nc",
    about = "Control noise cancellation on Bose headphones"
)]
struct Cli {
    /// Filter device by name substring (e.g. "NC700", "`QCUltra`")
    #[arg(short, long, global = true)]
    device: Option<String>,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show current noise cancellation status
    Status,

    /// Set noise cancellation level (range depends on device)
    Set {
        /// NC level (0 = transparency / minimum, max depends on device)
        level: u8,
    },

    /// Disable noise cancellation
    Off,

    /// List connected Bose devices
    Scan,

    /// Print current version and exit
    Version,
}

#[allow(clippy::print_stderr)]
fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };

    let result = match cli.command {
        Command::Scan => Ok(commands::run_scan()),
        Command::Status => commands::run_status(cli.device.as_deref()),
        Command::Set { level } => commands::run_set(cli.device.as_deref(), level),
        Command::Off => commands::run_off(cli.device.as_deref()),
        Command::Version => Ok(Report::Version(VersionReport {
            version: env!("CARGO_PKG_VERSION"),
        })),
    };

    match result.and_then(|report| output::print(report, format)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
