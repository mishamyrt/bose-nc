use std::ffi::{CString, c_char};
use thiserror::Error;

/// Errors that can occur when sending data via RFCOMM.
#[derive(Error, Debug)]
pub(crate) enum RfcommError {
    #[error("no response from device")]
    NoResponse,

    #[error("invalid response")]
    InvalidResponse,

    #[error("failed to open RFCOMM channel")]
    FailedToOpenChannel,

    #[error("failed to write to RFCOMM channel")]
    FailedToWrite,

    #[error("invalid bluetooth address: {0}")]
    InvalidAddress(String),
}

type RfcommResult<T> = Result<T, RfcommError>;

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

const RESPONSE_BUF_SIZE: usize = 256;

pub(crate) struct RfcommHandle {
    address: CString,
}

impl RfcommHandle {
    pub(crate) fn new(address: &str) -> RfcommResult<Self> {
        let address =
            CString::new(address).map_err(|_| RfcommError::InvalidAddress(address.into()))?;
        Ok(Self { address })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub(crate) fn send(&self, data: &[u8], repeat: i32) -> RfcommResult<()> {
        let n = unsafe {
            bose_rfcomm_send(
                self.address.as_ptr(),
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

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub(crate) fn send_receive(&self, data: &[u8]) -> RfcommResult<Vec<u8>> {
        let mut buf = [0u8; RESPONSE_BUF_SIZE];
        let n = unsafe {
            bose_rfcomm_send(
                self.address.as_ptr(),
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
            return Err(RfcommError::NoResponse);
        }

        #[allow(clippy::cast_sign_loss)]
        Ok(buf[..n as usize].to_vec())
    }
}

/// Check the result of an RFCOMM operation, returns data length or error if necessary.
fn check_rfcomm_result(result: i32) -> RfcommResult<i32> {
    match result {
        -1 => Err(RfcommError::NoResponse),
        -2 => Err(RfcommError::InvalidResponse),
        -3 => Err(RfcommError::FailedToOpenChannel),
        -4 => Err(RfcommError::FailedToWrite),
        _ => Ok(result),
    }
}
