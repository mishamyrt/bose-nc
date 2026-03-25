mod bmap;
mod device;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bose-cli",
    about = "Control noise cancellation on Bose headphones"
)]
struct Cli {
    /// Filter device by name substring (e.g. "NC700", "`QCUltra`")
    #[arg(short, long, global = true)]
    device: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show current noise cancellation status
    Status,

    /// Set noise cancellation level (0-10, where 10 = max NC)
    Set {
        /// NC level: 0 (transparency) to 10 (max noise cancelling)
        #[arg(value_parser = clap::value_parser!(u8).range(0..=10))]
        level: u8,
    },

    /// Disable noise cancellation
    Off,

    /// List connected Bose devices
    Scan,
}

#[allow(clippy::print_stdout)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan => {
            let devices = device::list_connected_bose();
            if devices.is_empty() {
                println!("No connected Bose devices found.");
            } else {
                for d in &devices {
                    println!("  {} ({})", d.name, d.address);
                }
            }
        }
        Command::Status => {
            let dev = device::find_device(cli.device.as_deref())?;
            let status = dev.get_nc_status()?;
            print_status(&status);
        }
        Command::Set { level } => {
            let dev = device::find_device(cli.device.as_deref())?;
            dev.set_nc(level, true)?;
            println!("Noise cancellation: ON (level {level})");
        }
        Command::Off => {
            let dev = device::find_device(cli.device.as_deref())?;
            dev.set_nc(0, false)?;
            println!("Noise cancellation: OFF");
        }
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_status(status: &bmap::CncStatus) {
    if status.enabled {
        println!(
            "Noise cancellation: ON (level {}/{})",
            status.level, status.max_level
        );
    } else {
        println!("Noise cancellation: OFF");
    }
}
