mod nc700;
mod qc35;
mod qc45;
mod qc_earbuds;
mod qc_ultra;
mod status;

pub(crate) use status::NcStatus;

use std::ffi::CString;

use anyhow::{Result, anyhow, bail};

use crate::bluetooth::{self, BluetoothDevice};
use crate::bmap;

const BOSE_VENDOR_ID: u16 = 0x009E;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Capability {
    NcGet,
    NcSet,
    NcOff,
}

impl Capability {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NcGet => "nc_get",
            Self::NcSet => "nc_set",
            Self::NcOff => "nc_off",
        }
    }
}

pub(crate) trait DeviceProfile {
    fn product_name(&self) -> &'static str;
    fn max_nc_level(&self) -> u8;
    fn capabilities(&self) -> &'static [Capability];

    fn build_nc_get(&self) -> Vec<u8>;
    fn build_nc_set(&self, level: u8) -> Vec<u8>;

    /// Build the "disable NC" packet. Returns `None` if the device has no
    /// dedicated off command (e.g. QC35, QC45, QC Ultra).
    fn build_nc_off(&self) -> Option<Vec<u8>> {
        None
    }

    fn parse_nc_status(&self, payload: &[u8]) -> Result<NcStatus>;

    fn nc_set_repeat_count(&self) -> i32 {
        1
    }
}

fn resolve_profile(product_id: u16) -> Option<Box<dyn DeviceProfile>> {
    match product_id {
        nc700::PRODUCT_ID => Some(Box::new(nc700::Nc700)),
        qc_ultra::PRODUCT_ID => Some(Box::new(qc_ultra::QcUltra)),
        qc35::PRODUCT_ID => Some(Box::new(qc35::Qc35)),
        qc45::PRODUCT_ID => Some(Box::new(qc45::Qc45)),
        qc_earbuds::PRODUCT_ID => Some(Box::new(qc_earbuds::QcEarbuds)),
        _ => None,
    }
}

pub(crate) struct ConnectedDevice {
    address: CString,
    profile: Box<dyn DeviceProfile>,
}

impl ConnectedDevice {
    fn new(address: &str, profile: Box<dyn DeviceProfile>) -> Result<Self> {
        Ok(Self {
            address: CString::new(address)
                .map_err(|_| anyhow!("invalid Bluetooth address: {address}"))?,
            profile,
        })
    }

    pub(crate) fn product_name(&self) -> &str {
        self.profile.product_name()
    }

    pub(crate) fn max_nc_level(&self) -> u8 {
        self.profile.max_nc_level()
    }

    pub(crate) fn get_nc_status(&self) -> Result<NcStatus> {
        let cmd = self.profile.build_nc_get();
        let data = bluetooth::rfcomm_send_receive(&self.address, &cmd)?;
        let packet = bmap::Packet::parse(&data)?;
        if packet.operator == bmap::OP_ERROR {
            bail!("device error: {:02x?}", packet.payload);
        }
        self.profile.parse_nc_status(&packet.payload)
    }

    pub(crate) fn set_nc(&self, level: u8) -> Result<()> {
        let cmd = self.profile.build_nc_set(level);
        bluetooth::rfcomm_send(&self.address, &cmd, self.profile.nc_set_repeat_count())
    }

    pub(crate) fn disable_nc(&self) -> Result<()> {
        let cmd = self.profile.build_nc_off().ok_or_else(|| {
            anyhow!(
                "{} does not support disabling noise cancellation",
                self.profile.product_name()
            )
        })?;
        bluetooth::rfcomm_send(&self.address, &cmd, self.profile.nc_set_repeat_count())
    }
}

/// Static information about a recognized Bose device (no connection needed).
pub(crate) struct DeviceInfo {
    pub(crate) product_name: &'static str,
    pub(crate) max_nc_level: u8,
    pub(crate) capabilities: &'static [Capability],
}

pub(crate) fn device_info(device: &BluetoothDevice) -> Option<DeviceInfo> {
    let profile = device.product_id.and_then(resolve_profile)?;
    Some(DeviceInfo {
        product_name: profile.product_name(),
        max_nc_level: profile.max_nc_level(),
        capabilities: profile.capabilities(),
    })
}

pub(crate) fn list_bose_devices() -> Vec<BluetoothDevice> {
    let Ok(devices) = bluetooth::list_connected_devices() else {
        return vec![];
    };
    devices
        .into_iter()
        .filter(|d| d.vendor_id == Some(BOSE_VENDOR_ID))
        .collect()
}

fn resolve_device_profile(device: &BluetoothDevice) -> Result<Box<dyn DeviceProfile>> {
    device.product_id.and_then(resolve_profile).ok_or_else(|| {
        anyhow!(
            "unsupported Bose product: {} (product_id: {:#06x})",
            device.name,
            device.product_id.unwrap_or(0)
        )
    })
}

#[allow(clippy::print_stderr)]
pub(crate) fn find_device(name_filter: Option<&str>) -> Result<ConnectedDevice> {
    let devices = list_bose_devices();

    if devices.is_empty() {
        return Err(anyhow!(
            "No paired Bose devices found. Pair your headphones in System Settings > Bluetooth."
        ));
    }

    let filtered: Vec<&BluetoothDevice> = devices
        .iter()
        .filter(|d| {
            name_filter
                .map(|f| d.name.to_lowercase().contains(&f.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    match filtered.len() {
        0 => Err(anyhow!(
            "No Bose device matches '{}'. Available: {}",
            name_filter.unwrap_or(""),
            devices
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        1 => {
            let d = filtered[0];
            let profile = resolve_device_profile(d)?;
            eprintln!("Using: {} ({})", d.name, d.address);
            ConnectedDevice::new(&d.address, profile)
        }
        _ => {
            eprintln!("Multiple Bose devices found:");
            for d in &filtered {
                eprintln!("  {} ({})", d.name, d.address);
            }
            let d = filtered[0];
            let profile = resolve_device_profile(d)?;
            eprintln!("Using first: {}", d.name);
            ConnectedDevice::new(&d.address, profile)
        }
    }
}
