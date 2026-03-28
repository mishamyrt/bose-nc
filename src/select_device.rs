use thiserror::Error;

use crate::{
    bluetooth::BluetoothDevice,
    device::{self, ConnectedDevice},
};

pub(crate) struct SelectedDevice {
    pub(crate) device: ConnectedDevice,
    pub(crate) notices: Vec<SelectionNotice>,
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

#[derive(Debug, Error)]
pub(crate) enum SelectDeviceError {
    #[error("No paired Bose devices found.")]
    NoDeviceFound,

    #[error("No Bose device matches '{filter}'. Available: {available}")]
    NoMatch { filter: String, available: String },

    #[error(transparent)]
    Device(#[from] device::DeviceError),
}

pub(crate) type Result<T> = std::result::Result<T, SelectDeviceError>;

pub(crate) fn find_device(name_filter: Option<&str>) -> Result<SelectedDevice> {
    let devices = device::list_bose_devices();

    if devices.is_empty() {
        return Err(SelectDeviceError::NoDeviceFound);
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
        0 => Err(SelectDeviceError::NoMatch {
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
            let matched = filtered.iter().map(|device| summarize(device)).collect();
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
