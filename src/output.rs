use serde_json::json;

use crate::error::AppError;

#[derive(Clone, Copy)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

pub(crate) enum Report {
    Scan(ScanReport),
    Status(StatusReport),
    Set(SetReport),
    Off(OffReport),
    Version(VersionReport),
}

pub(crate) struct ScanReport {
    pub(crate) items: Vec<ScanItem>,
}

pub(crate) struct ScanItem {
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) product: Option<String>,
    pub(crate) max_level: Option<u8>,
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

#[derive(Debug, Clone)]
pub(crate) enum SelectionNotice {
    MultipleMatches {
        matched: Vec<DeviceSummary>,
        selected: DeviceSummary,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct DeviceSummary {
    pub(crate) name: String,
    pub(crate) address: String,
}

pub(crate) fn print(report: Report, format: OutputFormat) -> Result<(), AppError> {
    match format {
        OutputFormat::Text => {
            print_text(report);
            Ok(())
        }
        OutputFormat::Json => print_json(report),
    }
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
fn print_text(report: Report) {
    match report {
        Report::Scan(report) => print_scan_text(&report.items),
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
}

#[allow(clippy::print_stdout)]
fn print_json(report: Report) -> Result<(), AppError> {
    let value = match report {
        Report::Scan(report) => {
            let items: Vec<_> = report
                .items
                .iter()
                .map(|item| {
                    let mut obj = json!({
                        "name": item.name,
                        "address": item.address,
                    });
                    if let Some(product) = &item.product {
                        let map = obj
                            .as_object_mut()
                            .expect("scan item should always be a JSON object");
                        map.insert("product".into(), json!(product));
                        map.insert(
                            "max_level".into(),
                            json!(item.max_level.unwrap_or_default()),
                        );
                        map.insert("capabilities".into(), json!(item.capabilities));
                    }
                    obj
                })
                .collect();
            json!(items)
        }
        Report::Status(report) => json!({
            "enabled": report.enabled,
            "level": report.level,
            "max_level": report.max_level,
        }),
        Report::Set(report) => json!({
            "level": report.level,
        }),
        Report::Off(_) => json!({
            "enabled": false,
        }),
        Report::Version(report) => json!({
            "version": report.version,
        }),
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
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
        if let Some(max_level) = item.max_level {
            println!("    NC levels: 0-{max_level}");
            println!("    Capabilities: {}", item.capabilities.join(", "));
        } else {
            println!("    (unsupported model)");
        }
    }
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
