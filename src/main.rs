mod bmap;
mod device;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bose-nc",
    about = "Control noise cancellation on Bose headphones"
)]
struct Cli {
    /// Filter device by name substring (e.g. "NC700", "QCUltra")
    #[arg(short, long)]
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
        level: u8,
    },

    /// Disable noise cancellation
    Off,

    /// List connected Bose devices
    Scan,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Command::Scan = cli.command {
        let devices = device::list_connected_bose()?;
        if devices.is_empty() {
            println!("No connected Bose devices found.");
        } else {
            for d in &devices {
                println!("  {} ({})", d.name, d.address);
            }
        }
        return Ok(());
    }

    let dev = device::find_device(cli.device.as_deref())?;

    match cli.command {
        Command::Status => {
            let status = dev.get_nc_status()?;
            print_status(&status);
        }
        Command::Set { level } => {
            if level > 10 {
                return Err(anyhow::anyhow!("Level must be 0-10"));
            }
            dev.set_nc(level, level > 0)?;
            println!(
                "Noise cancellation: {}",
                if level > 0 {
                    format!("ON (level {})", level)
                } else {
                    "OFF (transparency)".to_string()
                }
            );
        }
        Command::Off => {
            dev.set_nc(0, false)?;
            println!("Noise cancellation: OFF");
        }
        Command::Scan => unreachable!(),
    }

    Ok(())
}

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
