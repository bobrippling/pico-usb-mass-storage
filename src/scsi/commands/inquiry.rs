use overlay_macro::overlay;
use packing::Packed;
use crate::scsi::{
    packing::ParsePackedStruct,
    commands::Control,
};

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub struct InquiryCommand {
    #[bit_byte(7, 0, 0, 0)]
    pub op_code: u8,

    /// If set, return vital data related to the page_code field
    #[bit_byte(0, 0, 1, 1)]
    pub enable_vital_product_data: bool,

    /// What kind of vital data to return
    #[bit_byte(7, 0, 2, 2)]
    pub page_code: u8,

    ///TODO: (check) Should match data_transfer_length in CBW
    #[bit_byte(7, 0, 3, 4)]
    pub allocation_length: u16,

    #[bit_byte(7, 0, 5, 5)]
    pub control: u8, // TODO: `Control`
}

/*
 if evpd
    return data related to page_code (spc-4 section 7.8)
    if unsupported(page_code)
        return CHECK_CONDITION and set SENSE:
            key: ILLEGAL_REQUEST
            additional code: INVALID_FIELD_IN_CBD

 if !evpd
    return standard inquiry data (spc-4 section 6.4.2)
    if page_code != 0
        return CHECK_CONDITION and set SENSE:
            key: ILLEGAL_REQUEST
            additional code: INVALID_FIELD_IN_CBD
*/


/*
TODO: tests
#[test]
fn test_inquiry() {
    let mut bytes = [0; 5];
    let mut cmd = InquiryCommand::default();
    assert_eq!(cmd, InquiryCommand::unpack(&bytes).unwrap());

    bytes[0] |= 0b00000001;
    cmd.enable_vital_product_data = true;
    assert_eq!(cmd, InquiryCommand::unpack(&bytes).unwrap());

    bytes[1] = 0x99;
    cmd.page_code = 0x99;
    assert_eq!(cmd, InquiryCommand::unpack(&bytes).unwrap());

    let al = 9999;
    bytes[2] = ((al >> 8) & 0xFF) as u8;
    bytes[3] = ((al >> 0) & 0xFF) as u8;
    cmd.allocation_length = al;
    assert_eq!(cmd, InquiryCommand::unpack(&bytes).unwrap());

    bytes[4] = 0x31;
    cmd.control = Control::default();
    assert_eq!(cmd, InquiryCommand::unpack(&bytes).unwrap());
}
*/
