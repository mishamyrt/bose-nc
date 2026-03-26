#[derive(Debug)]
pub(crate) struct NcStatus {
    pub(crate) level: u8,
    pub(crate) max_level: u8,
    pub(crate) enabled: bool,
}
