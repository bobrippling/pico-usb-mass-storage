// There are many more variants (see asc-num.txt) but these are the ones the scsi code
// currently uses
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format, Default)]
pub enum AdditionalSenseCode {
    /// ASC 0x20, ASCQ: 0x0 - INVALID COMMAND OPERATION CODE
    InvalidCommandOperationCode,
    /// ASC 0x64, ASCQ: 0x1 - INVALID PACKET SIZE
    InvalidPacketSize,
    /// ASC 0x24, ASCQ: 0x0 - INVALID FIELD IN CDB
    InvalidFieldInCdb,
    /// ASC 0x0, ASCQ: 0x0 - NO ADDITIONAL SENSE INFORMATION
    #[default]
    NoAdditionalSenseInformation,
    /// ASC 0xC, ASCQ: 0x0 - WRITE ERROR
    WriteError,
    /// ASC 0x51, ASCQ: 0x0 - ERASE FAILURE
    EraseFailure,
    /// ASC 0x21, ASCQ: 0x0 - LOGICAL BLOCK ADDRESS OUT OF RANGE
    LogicalBlockAddressOutOfRange,
}

#[allow(dead_code)]
impl AdditionalSenseCode {
    /// Returns the ASC code for this variant
    pub fn asc(&self) -> u8 {
        match self {
            AdditionalSenseCode::InvalidCommandOperationCode => 32,
            AdditionalSenseCode::InvalidPacketSize => 100,
            AdditionalSenseCode::InvalidFieldInCdb => 36,
            AdditionalSenseCode::NoAdditionalSenseInformation => 0,
            AdditionalSenseCode::WriteError => 12,
            AdditionalSenseCode::EraseFailure => 81,
            AdditionalSenseCode::LogicalBlockAddressOutOfRange => 33,
        }
    }
    /// Returns the ASCQ code for this variant
    pub fn ascq(&self) -> u8 {
        match self {
            AdditionalSenseCode::InvalidCommandOperationCode => 0,
            AdditionalSenseCode::InvalidPacketSize => 1,
            AdditionalSenseCode::InvalidFieldInCdb => 0,
            AdditionalSenseCode::NoAdditionalSenseInformation => 0,
            AdditionalSenseCode::WriteError => 0,
            AdditionalSenseCode::EraseFailure => 0,
            AdditionalSenseCode::LogicalBlockAddressOutOfRange => 0,
        }
    }
    /// Returns the ASCQ code for this variant
    pub fn from(asc: u8, ascq: u8) -> core::option::Option<Self> {
        match (asc, ascq) {
            (32, 0) => Some(AdditionalSenseCode::InvalidCommandOperationCode),
            (100, 1) => Some(AdditionalSenseCode::InvalidPacketSize),
            (36, 0) => Some(AdditionalSenseCode::InvalidFieldInCdb),
            (0, 0) => Some(AdditionalSenseCode::NoAdditionalSenseInformation),
            (12, 0) => Some(AdditionalSenseCode::WriteError),
            (81, 0) => Some(AdditionalSenseCode::EraseFailure),
            (33, 0) => Some(AdditionalSenseCode::LogicalBlockAddressOutOfRange),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum AdditionalSenseCodeError {
    InvalidEnumDiscriminant,
}

impl TryFrom<u16> for AdditionalSenseCode {
    type Error = AdditionalSenseCodeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let asc = (value >> 8) as u8;
        let ascq = value as u8;

        Self::from(asc, ascq).ok_or(AdditionalSenseCodeError::InvalidEnumDiscriminant)
    }
}
