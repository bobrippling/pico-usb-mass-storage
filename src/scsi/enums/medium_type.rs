use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(TryFromPrimitive)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum MediumType {
    #[default]
    Sbc = 0x00,
}
