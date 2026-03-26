mod bluetooth;
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

    /// Set noise cancellation level (range depends on device)
    Set {
        /// NC level (0 = transparency / minimum, max depends on device)
        level: u8,
    },

    /// Disable noise cancellation
    Off,

    /// List connected Bose devices
    Scan {
        /// Output in JSON format for integration with other tools
        #[arg(long)]
        json: bool,
    },
}

#[allow(clippy::print_stdout)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { json } => {
            let devices = device::list_bose_devices();
            if json {
                print_scan_json(&devices);
            } else {
                print_scan_text(&devices);
            }
        }
        Command::Status => {
            let dev = device::find_device(cli.device.as_deref())?;
            let status = dev.get_nc_status()?;
            if status.enabled {
                println!(
                    "Noise cancellation: ON (level {}/{})",
                    status.level, status.max_level
                );
            } else {
                println!("Noise cancellation: OFF");
            }
        }
        Command::Set { level } => {
            let dev = device::find_device(cli.device.as_deref())?;
            let max = dev.max_nc_level();
            if level > max {
                anyhow::bail!(
                    "level {level} exceeds maximum ({max}) for {}",
                    dev.product_name()
                );
            }
            dev.set_nc(level)?;
            println!("Noise cancellation: ON (level {level})");
        }
        Command::Off => {
            let dev = device::find_device(cli.device.as_deref())?;
            dev.disable_nc()?;
            println!("Noise cancellation: OFF");
        }
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_scan_text(devices: &[bluetooth::BluetoothDevice]) {
    if devices.is_empty() {
        println!("No paired Bose devices found.");
        return;
    }
    for d in devices {
        println!("  {} ({})", d.name, d.address);
        if let Some(info) = device::device_info(d) {
            let caps: Vec<&str> = info.capabilities.iter().map(|c| c.as_str()).collect();
            println!("    NC levels: 0-{}", info.max_nc_level);
            println!("    Capabilities: {}", caps.join(", "));
        } else {
            println!("    (unsupported model)");
        }
    }
}

#[allow(clippy::print_stdout)]
fn print_scan_json(devices: &[bluetooth::BluetoothDevice]) {
    let items: Vec<serde_json::Value> = devices
        .iter()
        .map(|d| {
            let mut obj = serde_json::json!({
                "name": d.name,
                "address": d.address,
            });
            if let Some(info) = device::device_info(d) {
                let map = obj.as_object_mut().unwrap();
                map.insert(
                    "product".into(),
                    serde_json::Value::String(info.product_name.into()),
                );
                map.insert("max_nc_level".into(), info.max_nc_level.into());
                map.insert(
                    "capabilities".into(),
                    info.capabilities
                        .iter()
                        .map(|c| serde_json::Value::String(c.as_str().into()))
                        .collect(),
                );
            }
            obj
        })
        .collect();

    #[allow(clippy::print_stdout)]
    if let Ok(json) = serde_json::to_string_pretty(&items) {
        println!("{json}");
    }
}
