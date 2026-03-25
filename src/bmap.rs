#![allow(dead_code)]

//! Bose BMAP (Bose Multi-device Application Protocol) implementation.
//! Reverse-engineered from the Bose Music Android application.
//!
//! NC 700 wire format for `SettingsCnc`:
//!   Send: `[0x01, 0x05, 0x02, 0x02, (10 - level), enabled]`
//!   Response: `[0x01, 0x05, 0x03, 0x03, 0x0B, (10 - level), enabled]`
//!   Level is inverted: `wire_value = 10 - user_level`

pub(crate) const HEADER_SIZE: usize = 4;
const MAX_NC_LEVEL: u8 = 10;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum FunctionBlock {
    Settings = 0x01,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum Function {
    SettingsCnc = 0x05,
    SettingsAnr = 0x06,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum Operator {
    Set = 0x00,
    Get = 0x01,
    SetGet = 0x02,
    Status = 0x03,
    Error = 0x04,
}

#[derive(Debug)]
pub(crate) struct BmapPacket {
    pub(crate) function_block: u8,
    pub(crate) function: u8,
    pub(crate) device_id: u8,
    pub(crate) port_num: u8,
    pub(crate) operator: u8,
    pub(crate) payload: Vec<u8>,
}

impl BmapPacket {
    pub(crate) fn new(
        function_block: FunctionBlock,
        function: Function,
        operator: Operator,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            function_block: function_block as u8,
            function: function as u8,
            device_id: 0,
            port_num: 0,
            operator: operator as u8,
            payload,
        }
    }

    fn device_port_operator_byte(&self) -> u8 {
        (self.device_id << 6) | (self.port_num << 4) | self.operator
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.push(self.function_block);
        buf.push(self.function);
        buf.push(self.device_port_operator_byte());
        buf.push(self.payload.len() as u8);
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub(crate) fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < HEADER_SIZE {
            return None;
        }
        let payload_len = data[3] as usize;
        if data.len() < HEADER_SIZE + payload_len {
            return None;
        }
        Some(Self {
            function_block: data[0],
            function: data[1],
            device_id: (data[2] >> 6) & 0x03,
            port_num: (data[2] >> 4) & 0x03,
            operator: data[2] & 0x0F,
            payload: data[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct CncStatus {
    pub(crate) level: u8,
    pub(crate) max_level: u8,
    pub(crate) enabled: bool,
}

impl CncStatus {
    /// Parse CNC response payload: `[num_steps, 10-level, enabled]`.
    pub(crate) fn parse(payload: &[u8]) -> Option<Self> {
        if payload.len() < 3 {
            return None;
        }
        let num_steps = payload[0];
        let wire_level = payload[1];
        let enabled = payload[2] != 0;
        let max_level = num_steps.saturating_sub(1);
        let level = max_level.saturating_sub(wire_level);
        Some(Self {
            level,
            max_level,
            enabled,
        })
    }
}

/// Build a CNC Get packet (query current noise cancellation state).
pub(crate) fn cnc_get_packet() -> Vec<u8> {
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsCnc,
        Operator::Get,
        vec![],
    )
    .to_bytes()
}

/// Build a CNC `SetGet` packet for NC 700.
/// `level`: user-facing level 0-10 (10 = max NC, 0 = full transparency).
/// `enabled`: whether NC is on at all.
pub(crate) fn cnc_set_packet(level: u8, enabled: bool) -> Vec<u8> {
    let wire_level = MAX_NC_LEVEL.saturating_sub(level);
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsCnc,
        Operator::SetGet,
        vec![wire_level, u8::from(enabled)],
    )
    .to_bytes()
}

pub(crate) fn anr_set_packet(level: u8) -> Vec<u8> {
    BmapPacket::new(
        FunctionBlock::Settings,
        Function::SettingsAnr,
        Operator::SetGet,
        vec![level],
    )
    .to_bytes()
}
