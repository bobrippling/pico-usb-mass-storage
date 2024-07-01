use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(TryFromPrimitive)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum ResponseCode {
    #[default]
    FixedSenseData = 0x70,
    DescriptorSenseData = 0x72,
}
