mod bmap;
mod nc700;
mod qc35;
mod qc45;
mod qc_earbuds;
mod qc_ultra;
mod types;

use thiserror::Error;

pub(crate) use types::NcStatus;

use crate::bluetooth::{self, BluetoothDevice, RfcommError, RfcommHandle};

use types::Capability;

#[derive(Error, Debug)]
pub(crate) enum DeviceError {
    #[error("rfcomm error: {0}")]
    RfcommError(#[from] RfcommError),

    #[error("bmap protocol error: {0}")]
    BmapParsing(#[from] bmap::PacketError),

    #[error("device responded with error: {0}")]
    ResponseError(String),

    #[error("noise cancellation disable not supported by device")]
    NcDisableNotSupported,

    #[error("unsupported device with product id {0}")]
    UnsupportedDevice(u16),
}

pub(crate) type Result<T> = std::result::Result<T, DeviceError>;

const BOSE_VENDOR_ID: u16 = 0x009E;

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
    Some(match product_id {
        nc700::PRODUCT_ID => Box::new(nc700::Nc700),
        qc_ultra::PRODUCT_ID => Box::new(qc_ultra::QcUltra),
        qc35::PRODUCT_ID => Box::new(qc35::Qc35),
        qc45::PRODUCT_ID => Box::new(qc45::Qc45),
        qc_earbuds::PRODUCT_ID => Box::new(qc_earbuds::QcEarbuds),
        _ => return None,
    })
}

pub(crate) struct ConnectedDevice {
    handle: RfcommHandle,
    profile: Box<dyn DeviceProfile>,
}

impl ConnectedDevice {
    fn new(address: &str, profile: Box<dyn DeviceProfile>) -> Result<Self> {
        Ok(Self {
            handle: RfcommHandle::new(address)?,
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
        let data = self.handle.send_receive(&cmd)?;
        let packet = bmap::Packet::parse(&data)?;
        if packet.operator == bmap::OP_ERROR {
            let response_bytes = format!("{:02x?}", packet.payload);
            return Err(DeviceError::ResponseError(response_bytes));
        }
        self.profile.parse_nc_status(&packet.payload)
    }

    pub(crate) fn set_nc(&self, level: u8) -> Result<()> {
        let cmd = self.profile.build_nc_set(level);
        self.handle.send(&cmd, self.profile.nc_set_repeat_count())?;
        Ok(())
    }

    pub(crate) fn disable_nc(&self) -> Result<()> {
        let cmd = self
            .profile
            .build_nc_off()
            .ok_or_else(|| DeviceError::NcDisableNotSupported)?;
        self.handle.send(&cmd, self.profile.nc_set_repeat_count())?;
        Ok(())
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

pub(crate) fn connect_device(device: &BluetoothDevice) -> Result<ConnectedDevice> {
    let profile = resolve_device_profile(device)?;
    ConnectedDevice::new(&device.address, profile)
}

fn resolve_device_profile(device: &BluetoothDevice) -> Result<Box<dyn DeviceProfile>> {
    device
        .product_id
        .and_then(resolve_profile)
        .ok_or_else(|| DeviceError::UnsupportedDevice(device.product_id.unwrap_or(0)))
}
