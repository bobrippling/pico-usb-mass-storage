//! USB Mass Storage transports

use core::fmt::Debug;
use defmt::Format;
use embassy_usb::driver::EndpointError;

#[cfg(feature = "bbb")]
pub mod bbb;

/// Interface protocol for specific transports
pub const TRANSPORT_VENDOR_SPECIFIC: u8 = 0xFF;

/// USB Mass Storage transport.
///
/// An implementation of this trait can be used as an underlying transport for subclasses
/// defined in [subclass] module .
///
/// [subclass]: crate::subclass
pub trait Transport {
    /// Interface protocol code
    const PROTO: u8;
}

/// Generic error type that could be used by [Transport] impls.
#[derive(Debug, Format)]
pub enum TransportError<E: Debug> {
    /// USB stack error
    Usb(EndpointError),
    /// Transport-specific error
    Error(E),
}

/// The status of a Mass Storage command.
///
/// Refer to the USB-MS doc.
#[repr(u8)]
#[derive(Default, Copy, Clone, Format)]
pub enum CommandStatus {
    #[default]
    Passed = 0x00,
    Failed = 0x01,
    PhaseError = 0x02,
}