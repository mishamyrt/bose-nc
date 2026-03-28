use anyhow::Result;
use serde_json::{Value, json};

use crate::select_device::SelectionNotice;

pub(crate) enum Report {
    Scan(ScanReport),
    Status(StatusReport),
    Set(SetReport),
    Off(OffReport),
    Version(VersionReport),
}

pub(crate) struct ScanReport {
    pub(crate) format: ScanFormat,
    pub(crate) items: Vec<ScanItem>,
}

pub(crate) enum ScanFormat {
    Text,
    Json,
}

pub(crate) struct ScanItem {
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) product: Option<String>,
    pub(crate) max_nc_level: Option<u8>,
    pub(crate) capabilities: Vec<String>,
}

pub(crate) struct StatusReport {
    pub(crate) notices: Vec<SelectionNotice>,
    pub(crate) enabled: bool,
    pub(crate) level: u8,
    pub(crate) max_level: u8,
}

pub(crate) struct SetReport {
    pub(crate) notices: Vec<SelectionNotice>,
    pub(crate) level: u8,
}

pub(crate) struct OffReport {
    pub(crate) notices: Vec<SelectionNotice>,
}

pub(crate) struct VersionReport {
    pub(crate) version: &'static str,
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
pub(crate) fn print(report: Report) -> Result<()> {
    match report {
        Report::Scan(report) => print_scan(report)?,
        Report::Status(report) => {
            print_notices(&report.notices);
            if report.enabled {
                println!(
                    "Noise cancellation: ON (level {}/{})",
                    report.level, report.max_level
                );
            } else {
                println!("Noise cancellation: OFF");
            }
        }
        Report::Set(report) => {
            print_notices(&report.notices);
            println!("Noise cancellation: ON (level {})", report.level);
        }
        Report::Off(report) => {
            print_notices(&report.notices);
            println!("Noise cancellation: OFF");
        }
        Report::Version(report) => println!("bose-nc {}", report.version),
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_scan(report: ScanReport) -> Result<()> {
    match report.format {
        ScanFormat::Text => print_scan_text(&report.items),
        ScanFormat::Json => print_scan_json(&report.items)?,
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_scan_text(items: &[ScanItem]) {
    if items.is_empty() {
        println!("No connected Bose devices found.");
        return;
    }

    for item in items {
        println!("  {} ({})", item.name, item.address);
        if let Some(max_nc_level) = item.max_nc_level {
            println!("    NC levels: 0-{max_nc_level}");
            println!("    Capabilities: {}", item.capabilities.join(", "));
        } else {
            println!("    (unsupported model)");
        }
    }
}

#[allow(clippy::print_stdout)]
fn print_scan_json(items: &[ScanItem]) -> Result<()> {
    let items: Vec<Value> = items
        .iter()
        .map(|item| {
            let mut object = json!({
                "name": item.name,
                "address": item.address,
            });

            if let Some(product) = &item.product {
                let map = object
                    .as_object_mut()
                    .expect("scan report items should always be JSON objects");
                map.insert("product".into(), Value::String(product.clone()));
                map.insert(
                    "max_nc_level".into(),
                    item.max_nc_level.unwrap_or_default().into(),
                );
                map.insert(
                    "capabilities".into(),
                    item.capabilities
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                );
            }

            object
        })
        .collect();

    let json = serde_json::to_string_pretty(&items)?;
    println!("{json}");

    Ok(())
}

#[allow(clippy::print_stderr)]
fn print_notices(notices: &[SelectionNotice]) {
    for notice in notices {
        match notice {
            SelectionNotice::MultipleMatches { matched, selected } => {
                eprintln!("Multiple Bose devices found:");
                for device in matched {
                    eprintln!("  {} ({})", device.name, device.address);
                }
                eprintln!("Using first: {}", selected.name);
            }
        }
    }
}
