use crate::{
    bluetooth::BluetoothDevice,
    device::{self, ConnectedDevice},
    error::AppError,
    output::{
        DeviceSummary, OffReport, Report, ScanItem, ScanReport, SelectionNotice, SetReport,
        StatusReport,
    },
};

struct SelectedDevice {
    device: ConnectedDevice,
    notices: Vec<SelectionNotice>,
}

pub(crate) fn run_scan() -> Report {
    let items = device::list_bose_devices()
        .into_iter()
        .map(|d| {
            let info = device::device_info(&d);
            ScanItem {
                name: d.name,
                address: d.address,
                product: info.as_ref().map(|i| i.product_name.to_owned()),
                max_level: info.as_ref().map(|i| i.max_nc_level),
                capabilities: info
                    .map(|i| {
                        i.capabilities
                            .iter()
                            .map(|c| c.as_str().to_owned())
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();

    Report::Scan(ScanReport { items })
}

pub(crate) fn run_status(name_filter: Option<&str>) -> Result<Report, AppError> {
    let selected = find_device(name_filter)?;
    let status = selected.device.get_nc_status()?;

    Ok(Report::Status(StatusReport {
        notices: selected.notices,
        enabled: status.enabled,
        level: status.level,
        max_level: status.max_level,
    }))
}

pub(crate) fn run_set(name_filter: Option<&str>, level: u8) -> Result<Report, AppError> {
    let selected = find_device(name_filter)?;
    let max = selected.device.max_nc_level();
    if level > max {
        return Err(AppError::NcLevelExceeded {
            level,
            max,
            product: selected.device.product_name().to_owned(),
        });
    }

    selected.device.set_nc(level)?;

    Ok(Report::Set(SetReport {
        notices: selected.notices,
        level,
    }))
}

pub(crate) fn run_off(name_filter: Option<&str>) -> Result<Report, AppError> {
    let selected = find_device(name_filter)?;
    selected.device.disable_nc()?;

    Ok(Report::Off(OffReport {
        notices: selected.notices,
    }))
}

fn find_device(name_filter: Option<&str>) -> Result<SelectedDevice, AppError> {
    let devices = device::list_bose_devices();

    if devices.is_empty() {
        return Err(AppError::NoDeviceFound);
    }

    let normalized_filter = name_filter.map(str::to_lowercase);
    let filtered: Vec<&BluetoothDevice> = devices
        .iter()
        .filter(|device| {
            normalized_filter
                .as_deref()
                .map(|filter| device.name.to_lowercase().contains(filter))
                .unwrap_or(true)
        })
        .collect();

    match filtered.len() {
        0 => Err(AppError::NoMatch {
            filter: name_filter.unwrap_or_default().to_owned(),
            available: devices
                .iter()
                .map(|device| device.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }),
        1 => Ok(SelectedDevice {
            device: device::connect_device(filtered[0])?,
            notices: vec![],
        }),
        _ => {
            let matched = filtered.iter().map(|d| summarize(d)).collect();
            let selected = summarize(filtered[0]);

            Ok(SelectedDevice {
                device: device::connect_device(filtered[0])?,
                notices: vec![SelectionNotice::MultipleMatches { matched, selected }],
            })
        }
    }
}

fn summarize(device: &BluetoothDevice) -> DeviceSummary {
    DeviceSummary {
        name: device.name.clone(),
        address: device.address.clone(),
    }
}
