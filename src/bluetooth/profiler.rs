use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct BluetoothDevice {
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) vendor_id: Option<u16>,
    pub(crate) product_id: Option<u16>,
}

pub(crate) fn list_connected_devices() -> Result<Vec<BluetoothDevice>> {
    let output = Command::new("system_profiler")
        .args(["-json", "SPBluetoothDataType"])
        .output()
        .context("failed to run system_profiler")?;

    if !output.status.success() {
        anyhow::bail!("system_profiler exited with {}", output.status);
    }

    let json: Value =
        serde_json::from_slice(&output.stdout).context("failed to parse system_profiler JSON")?;

    let entries = json["SPBluetoothDataType"]
        .as_array()
        .context("unexpected system_profiler schema: missing SPBluetoothDataType")?;

    let mut devices = Vec::new();

    for entry in entries {
        // check if entry contains device_connected
        let devices_entry = entry["device_connected"].as_array();
        let Some(raw_devices) = devices_entry else {
            continue;
        };
        for raw_device in raw_devices {
            if let Some(device) = parse_device(raw_device) {
                devices.push(device);
            }
        }
    }

    Ok(devices)
}

fn parse_device(raw: &Value) -> Option<BluetoothDevice> {
    let obj = raw.as_object()?;
    let (name, props) = obj.into_iter().next()?;
    let vendor_id = props["device_vendorID"].as_str().and_then(parse_id);
    let product_id = props["device_productID"].as_str().and_then(parse_id);

    Some(BluetoothDevice {
        name: name.clone(),
        address: props["device_address"]
            .as_str()
            .unwrap_or("")
            .to_uppercase(),
        product_id,
        vendor_id,
    })
}

fn parse_id(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}
