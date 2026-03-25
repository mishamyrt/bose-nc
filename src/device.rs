use std::ffi::{CString, c_char};

use anyhow::{Result, anyhow};

use crate::bmap::{self, BmapPacket, CncStatus, Operator};

const RESPONSE_BUF_SIZE: usize = 256;

unsafe extern "C" {
    fn bose_rfcomm_send(
        bt_address: *const c_char,
        send_buf: *const u8,
        send_len: i32,
        out_buf: *mut u8,
        out_capacity: i32,
        rfcomm_channel: i32,
        send_count: i32,
    ) -> i32;

    fn bose_list_devices(out_buf: *mut c_char, out_capacity: i32) -> i32;
}

fn check_rfcomm_result(code: i32) -> Result<i32> {
    match code {
        -1 => Err(anyhow!("Device not found")),
        -2 => Err(anyhow!("SPP service not found on device")),
        -3 => Err(anyhow!("Failed to open RFCOMM channel")),
        -4 => Err(anyhow!("Failed to write to RFCOMM channel")),
        n => Ok(n),
    }
}

pub(crate) struct BoseDevice {
    address: CString,
}

impl BoseDevice {
    pub(crate) fn new(address: &str) -> Result<Self> {
        Ok(Self {
            address: CString::new(address)
                .map_err(|_| anyhow!("invalid Bluetooth address: {address}"))?,
        })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn send_and_receive(&self, packet: &[u8]) -> Result<BmapPacket> {
        let mut response = [0u8; RESPONSE_BUF_SIZE];
        let received = unsafe {
            bose_rfcomm_send(
                self.address.as_ptr(),
                packet.as_ptr(),
                packet.len() as i32,
                response.as_mut_ptr(),
                RESPONSE_BUF_SIZE as i32,
                -1,
                1,
            )
        };

        let received = check_rfcomm_result(received)?;
        if received == 0 {
            return Err(anyhow!("No response from device"));
        }

        #[allow(clippy::cast_sign_loss)]
        let len = received as usize;
        BmapPacket::parse(&response[..len])
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn send_command(&self, packet: &[u8], send_count: i32) -> Result<()> {
        let received = unsafe {
            bose_rfcomm_send(
                self.address.as_ptr(),
                packet.as_ptr(),
                packet.len() as i32,
                std::ptr::null_mut(),
                0,
                -1,
                send_count,
            )
        };
        check_rfcomm_result(received)?;
        Ok(())
    }

    pub(crate) fn get_nc_status(&self) -> Result<CncStatus> {
        let cmd = bmap::cnc_get_packet();
        let packet = self.send_and_receive(&cmd)?;

        if packet.operator == Operator::Error {
            return Err(anyhow!("Device error: {:02x?}", packet.payload));
        }

        CncStatus::parse(&packet.payload)
    }

    pub(crate) fn set_nc(&self, level: u8, enabled: bool) -> Result<()> {
        let cmd = bmap::cnc_set_packet(level, enabled);
        // Send twice in one session: the device restores the previous level when
        // re-enabling NC, so the second write applies the correct level.
        self.send_command(&cmd, 2)
    }
}

#[derive(Debug)]
pub(crate) struct PairedDevice {
    pub(crate) address: String,
    pub(crate) name: String,
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn list_connected_bose() -> Vec<PairedDevice> {
    let mut buf = vec![0u8; 4096];
    let count = unsafe { bose_list_devices(buf.as_mut_ptr().cast::<c_char>(), buf.len() as i32) };

    if count <= 0 {
        return vec![];
    }

    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let text = String::from_utf8_lossy(&buf[..end]);
    let mut seen = std::collections::HashSet::new();
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\t');
            let address = parts.next()?.to_string();
            let name = parts.next()?.to_string();
            if seen.insert(address.clone()) {
                Some(PairedDevice { address, name })
            } else {
                None
            }
        })
        .collect()
}

#[allow(clippy::print_stderr)]
pub(crate) fn find_device(name_filter: Option<&str>) -> Result<BoseDevice> {
    let devices = list_connected_bose();

    if devices.is_empty() {
        return Err(anyhow!(
            "No paired Bose devices found. Pair your headphones in System Settings > Bluetooth."
        ));
    }

    let filtered: Vec<&PairedDevice> = devices
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
            eprintln!("Using: {} ({})", d.name, d.address);
            BoseDevice::new(&d.address)
        }
        _ => {
            eprintln!("Multiple Bose devices found:");
            for d in &filtered {
                eprintln!("  {} ({})", d.name, d.address);
            }
            let d = filtered[0];
            eprintln!("Using first: {}", d.name);
            BoseDevice::new(&d.address)
        }
    }
}
