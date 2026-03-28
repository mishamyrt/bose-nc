use std::process::Command;

use serde_json::Value;
use thiserror::Error;

const SP_PATH: &str = "/usr/sbin/system_profiler";
const SP_BLUETOOTH_TYPE: &str = "SPBluetoothDataType";

/// Errors that can occur when running `system_profiler` to list Bluetooth devices.
#[derive(Error, Debug)]
pub(crate) enum SystemProfilerError {
    #[error("failed to run system_profiler: {0}")]
    FailedToRun(#[from] std::io::Error),

    #[error("system_profiler exited with {0}")]
    ProcessError(std::process::ExitStatus),

    #[error("failed to parse system_profiler JSON")]
    ParseError(#[from] serde_json::Error),

    #[error("unexpected system_profiler schema: SPBluetoothDataType not found")]
    TypeNotFound,

    #[error("unexpected system_profiler schema: expected 1 entry, got {0}")]
    WrongDataCount(usize),
}

#[derive(Debug)]
/// Represents a Bluetooth device information.
pub(crate) struct BluetoothDevice {
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) vendor_id: Option<u16>,
    pub(crate) product_id: Option<u16>,
}

/// Lists connected Bluetooth devices using `system_profiler`.
pub(crate) fn list_connected_devices() -> Result<Vec<BluetoothDevice>, SystemProfilerError> {
    let output = Command::new(SP_PATH)
        .args(["-json", SP_BLUETOOTH_TYPE])
        .output()?;
    if !output.status.success() {
        return Err(SystemProfilerError::ProcessError(output.status));
    }

    let json: Value = serde_json::from_slice(&output.stdout)?;

    let Some(entries) = json[SP_BLUETOOTH_TYPE].as_array() else {
        return Err(SystemProfilerError::TypeNotFound);
    };

    if entries.len() != 1 {
        return Err(SystemProfilerError::WrongDataCount(entries.len()));
    }
    let entry = &entries[0];

    let mut devices = Vec::new();
    let Some(raw_devices) = entry["device_connected"].as_array() else {
        return Ok(devices);
    };
    for raw_device in raw_devices {
        if let Some(device) = parse_device(raw_device) {
            devices.push(device);
        }
    }

    Ok(devices)
}

/// Parses a raw device entry into a [`BluetoothDevice`] struct.
fn parse_device(raw: &Value) -> Option<BluetoothDevice> {
    let obj = raw.as_object()?;
    let (name, props) = obj.into_iter().next()?;
    let Some(address) = props["device_address"].as_str().map(String::from) else {
        return None;
    };
    let vendor_id = props["device_vendorID"].as_str().and_then(parse_id);
    let product_id = props["device_productID"].as_str().and_then(parse_id);

    Some(BluetoothDevice {
        name: name.clone(),
        address,
        product_id,
        vendor_id,
    })
}

/// Parses a hexadecimal string into an `u16` value.
fn parse_id(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}
