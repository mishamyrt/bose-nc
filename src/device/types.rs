#[derive(Debug)]
pub(crate) struct NcStatus {
    pub(crate) level: u8,
    pub(crate) max_level: u8,
    pub(crate) enabled: bool,
}

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
