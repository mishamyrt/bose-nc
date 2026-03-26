mod profiler;

use std::ffi::{CStr, c_char};

use anyhow::{Result, anyhow};

pub(crate) use profiler::{BluetoothDevice, list_connected_devices};

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

const RESPONSE_BUF_SIZE: usize = 256;

/// Open an RFCOMM session, send `data`, and return the raw response bytes.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn rfcomm_send_receive(address: &CStr, data: &[u8]) -> Result<Vec<u8>> {
    let mut buf = [0u8; RESPONSE_BUF_SIZE];
    let n = unsafe {
        bose_rfcomm_send(
            address.as_ptr(),
            data.as_ptr(),
            data.len() as i32,
            buf.as_mut_ptr(),
            RESPONSE_BUF_SIZE as i32,
            -1,
            1,
        )
    };
    let n = check_rfcomm_result(n)?;
    if n == 0 {
        return Err(anyhow!("No response from device"));
    }
    #[allow(clippy::cast_sign_loss)]
    Ok(buf[..n as usize].to_vec())
}

/// Open an RFCOMM session and send `data` (repeated `repeat` times), ignoring any response.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn rfcomm_send(address: &CStr, data: &[u8], repeat: i32) -> Result<()> {
    let n = unsafe {
        bose_rfcomm_send(
            address.as_ptr(),
            data.as_ptr(),
            data.len() as i32,
            std::ptr::null_mut(),
            0,
            -1,
            repeat,
        )
    };
    check_rfcomm_result(n)?;
    Ok(())
}
