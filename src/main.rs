mod bluetooth;
mod device;
mod reports;
mod select_device;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use crate::{
    device::{device_info, list_bose_devices},
    reports::{
        OffReport, Report, ScanFormat, ScanItem, ScanReport, SetReport, StatusReport, VersionReport,
    },
    select_device::find_device,
};

#[derive(Parser)]
#[command(
    name = "bose-nc",
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

    /// Print current version and exit
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let report = match cli.command {
        Command::Scan { json } => Ok(run_scan(if json {
            ScanFormat::Json
        } else {
            ScanFormat::Text
        })),
        Command::Status => run_status(cli.device.as_deref()),
        Command::Set { level } => run_set(cli.device.as_deref(), level),
        Command::Off => run_off(cli.device.as_deref()),
        Command::Version => Ok(Report::Version(VersionReport {
            version: env!("CARGO_PKG_VERSION"),
        })),
    }?;
    reports::print(report)
}

fn run_scan(format: ScanFormat) -> Report {
    let items = list_bose_devices()
        .into_iter()
        .map(|device| {
            let info = device_info(&device);
            ScanItem {
                name: device.name,
                address: device.address,
                product: info.as_ref().map(|info| info.product_name.to_owned()),
                max_nc_level: info.as_ref().map(|info| info.max_nc_level),
                capabilities: info
                    .map(|info| {
                        info.capabilities
                            .iter()
                            .map(|capability| capability.as_str().to_owned())
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();

    Report::Scan(ScanReport { format, items })
}

fn run_status(name_filter: Option<&str>) -> Result<Report> {
    let selected = find_device(name_filter)?;
    let status = selected.device.get_nc_status()?;

    Ok(Report::Status(StatusReport {
        notices: selected.notices,
        enabled: status.enabled,
        level: status.level,
        max_level: status.max_level,
    }))
}

fn run_set(name_filter: Option<&str>, level: u8) -> Result<Report> {
    let selected = find_device(name_filter)?;
    let max_level = selected.device.max_nc_level();
    if level > max_level {
        bail!(
            "level {level} exceeds maximum ({max_level}) for {}",
            selected.device.product_name()
        );
    }

    selected.device.set_nc(level)?;

    Ok(Report::Set(SetReport {
        notices: selected.notices,
        level,
    }))
}

fn run_off(name_filter: Option<&str>) -> Result<Report> {
    let selected = find_device(name_filter)?;
    selected.device.disable_nc()?;

    Ok(Report::Off(OffReport {
        notices: selected.notices,
    }))
}
