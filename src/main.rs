#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use bsp::entry;
use defmt::{info, warn, error};
use defmt_rtt as _;
use panic_probe as _;

use rp_pico as bsp;

use bsp::hal::{clocks::init_clocks_and_plls, pac, sio::Sio, watchdog::Watchdog};

use usb_device::{class_prelude::*, prelude::*};
use usbd_storage::{
    subclass::{
        scsi::{Scsi, ScsiCommand},
        Command,
    },
    transport::{
        bbb::{BulkOnly, BulkOnlyError},
        TransportError,
    },
};

/// Not necessarily `'static`. May reside in some special memory location
static mut USB_TRANSPORT_BUF: MaybeUninit<[u8; TRANSPORT_BUF_SIZE]> = MaybeUninit::uninit();
static mut STORAGE: [u8; (BLOCKS * BLOCK_SIZE) as usize] = [0u8; (BLOCK_SIZE * BLOCKS) as usize];

static mut STATE: State = State {
    storage_offset: 0,
    sense_key: None,
    sense_key_code: None,
    sense_qualifier: None,
};

const TRANSPORT_BUF_SIZE: usize = 512; // TODO: check
const BLOCK_SIZE: u32 = 4096; // TODO: check
const BLOCKS: u32 = 102400 / BLOCK_SIZE;
const USB_PACKET_SIZE: u16 = 64; // 8,16,32,64
const MAX_LUN: u8 = 0; // max 0x0F

#[derive(Clone, Default)]
struct State {
    storage_offset: usize,
    sense_key: Option<u8>,
    sense_key_code: Option<u8>,
    sense_qualifier: Option<u8>,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    defmt::timestamp!("{=u32}", unsafe { &*pac::TIMER::PTR }.timerawl.read().bits());

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // work around errata 5
    let sio = Sio::new(pac.SIO);
    let _pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let usb_bus = UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut scsi =
        usbd_storage::subclass::scsi::Scsi::new(&usb_bus, USB_PACKET_SIZE, MAX_LUN, unsafe {
            USB_TRANSPORT_BUF.assume_init_mut().as_mut_slice()
        })
        .unwrap();

    let mut usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0xabcd, 0xabcd))
        .manufacturer("Chris Price")
        .product("100k of your finest bytes")
        .serial_number("CP4096OYFB")
        .self_powered(false)
        .build();

    ch_flags::hacky_set();

    loop {
        if !usb_device.poll(&mut [&mut scsi]) {
            continue;
        }

        // clear state if just configured or reset
        if matches!(usb_device.state(), UsbDeviceState::Default) {
            unsafe {
                STATE.reset();
            };
        }

        let mut n = 0;
        let _ = scsi.poll(|command| {
            n += 1;
            if let Err(err) = process_command(command) {
                error!("{}", err);
            }
        });
        if n > 1 {
            warn!("poll called back {} times", n);
        }
    }
}

mod ch_flags {
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[repr(C)]
    struct Channel {
        pub name: *const u8,
        pub buffer: *mut u8,
        pub size: usize,
        pub write: AtomicUsize,
        pub read: AtomicUsize,
        pub flags: AtomicUsize,
    }

    impl defmt::Format for Channel {
        fn format(&self, fmt: defmt::Formatter<'_>) {
            use defmt::write;

            write!(
                fmt,
                "Channel {{
                    name: {:?},
                    buffer: ...,
                    size: {},
                    write: {},
                    read: {},
                    flags: {},
                }}",
                self.name,
                self.size,
                self.write.load(Ordering::Relaxed),
                self.read.load(Ordering::Relaxed),
                self.flags.load(Ordering::Relaxed),
            );
        }
    }

    #[repr(C)]
    struct Header {
        id: [u8; 16],
        max_up_channels: usize,
        max_down_channels: usize,
        up_channel: Channel,
    }

    impl defmt::Format for Header {
        fn format(&self, fmt: defmt::Formatter<'_>) {
            use defmt::write;

            write!(
                fmt,
                "Header {{
                    id: ...,
                    max_up_channels: {},
                    max_down_channels: {},
                    up_channel: {},
                }}",
                self.max_up_channels,
                self.max_down_channels,
                self.up_channel,
            );
        }
    }

    extern "C" {
        static mut _SEGGER_RTT: Header;
    }

    //const MODE_MASK: usize = 0b11;
    const MODE_BLOCK_IF_FULL: usize = 2;
    //const MODE_NON_BLOCKING_TRIM: usize = 1;

    pub fn hacky_set() {
        unsafe {
            let p = &_SEGGER_RTT.up_channel.flags;

            p.store(p.load(Ordering::Relaxed) & !MODE_BLOCK_IF_FULL, Ordering::Relaxed);
            super::warn!("hack'd the channel bit");
            super::warn!("dump of header: {:?}", _SEGGER_RTT);
        }
    }

    pub fn get() -> usize {
        unsafe {
            let p = &_SEGGER_RTT.up_channel.flags;
            p.load(Ordering::Relaxed)
        }
    }
}

fn process_command(
    mut command: Command<ScsiCommand, Scsi<BulkOnly<bsp::hal::usb::UsbBus, &mut [u8]>>>,
) -> Result<(), TransportError<BulkOnlyError>> {
    //info!("Handling: {}", command.kind);

    match command.kind {
        ScsiCommand::TestUnitReady { .. } => {
            command.pass();
        }
        ScsiCommand::Inquiry { .. } => {
            command.try_write_data_all(&[
                0x00, // periph qualifier, periph device type
                0x80, // Removable
                0x04, // SPC-2 compliance
                0x02, // NormACA, HiSu, Response data format
                0x20, // 36 bytes in total
                0x00, // additional fields, none set
                0x00, // additional fields, none set
                0x00, // additional fields, none set
                b'C', b'H', b'R', b'I', b'S', b'P', b' ', b' ', // 8-byte T-10 vendor id
                b'1', b'0', b'0', b'k', b' ', b'o', b'f', b' ', b'y', b'o', b'u', b'r', b' ', b'f',
                b'i', b'n', // 16-byte product identification
                b'1', b'.', b'2', b'3', // 4-byte product revision
            ])?;
            command.pass();
        }
        ScsiCommand::RequestSense { .. } => unsafe {
            command.try_write_data_all(&[
                0x70,                         // RESPONSE CODE. Set to 70h for information on current errors
                0x00,                         // obsolete
                STATE.sense_key.unwrap_or(0), // Bits 3..0: SENSE KEY. Contains information describing the error.
                0x00,
                0x00,
                0x00,
                0x00, // INFORMATION. Device-specific or command-specific information.
                0x00, // ADDITIONAL SENSE LENGTH.
                0x00,
                0x00,
                0x00,
                0x00,                               // COMMAND-SPECIFIC INFORMATION
                STATE.sense_key_code.unwrap_or(0),  // ASC
                STATE.sense_qualifier.unwrap_or(0), // ASCQ
                0x00,
                0x00,
                0x00,
                0x00,
            ])?;
            STATE.reset();
            command.pass();
        },
        ScsiCommand::ReadCapacity10 { .. } => {
            let mut data = [0u8; 8];
            let _ = &mut data[0..4].copy_from_slice(&(BLOCKS - 1).to_be_bytes());
            let _ = &mut data[4..8].copy_from_slice(&BLOCK_SIZE.to_be_bytes());
            command.try_write_data_all(&data)?;
            command.pass();
        }
        ScsiCommand::ReadCapacity16 { .. } => {
            let mut data = [0u8; 16];
            let _ = &mut data[0..8].copy_from_slice(&(BLOCKS - 1).to_be_bytes());
            let _ = &mut data[8..12].copy_from_slice(&BLOCK_SIZE.to_be_bytes());
            command.try_write_data_all(&data)?;
            command.pass();
        }
        ScsiCommand::ReadFormatCapacities { .. } => {
            let mut data = [0u8; 12];
            let _ = &mut data[0..4].copy_from_slice(&[
                0x00, 0x00, 0x00, 0x08, // capacity list length
            ]);
            let _ = &mut data[4..8].copy_from_slice(&(BLOCKS as u32).to_be_bytes()); // number of blocks
            data[8] = 0x01; //unformatted media
            let block_length_be = BLOCK_SIZE.to_be_bytes();
            data[9] = block_length_be[1];
            data[10] = block_length_be[2];
            data[11] = block_length_be[3];

            command.try_write_data_all(&data)?;
            command.pass();
        }
        ScsiCommand::Read { lba, len } => unsafe {
            let lba = lba as u32; // u64 -> u32
            let len = len as u32;
            if STATE.storage_offset != (len * BLOCK_SIZE) as usize {
                let start = (BLOCK_SIZE * lba) as usize + STATE.storage_offset;
                let end = (BLOCK_SIZE * lba) as usize + (BLOCK_SIZE * len) as usize;

                // Uncomment this in order to push data in chunks smaller than a USB packet.
                // let end = min(start + USB_PACKET_SIZE as usize - 1, end);

                info!("Data transfer >>>>>>>> [{}..{}]", start, end);

                let count = command.write_data(&STORAGE[start..end])?;
                STATE.storage_offset += count;
            } else {
                command.pass();
                STATE.storage_offset = 0;
            }
        },
        ScsiCommand::Write { lba, len } => unsafe {
            let lba = lba as u32;
            let len = len as u32;
            if STATE.storage_offset != (len * BLOCK_SIZE) as usize {
                let start = (BLOCK_SIZE * lba) as usize + STATE.storage_offset;
                let end = (BLOCK_SIZE * lba) as usize + (BLOCK_SIZE * len) as usize;

                //info!("Data transfer <<<<<<<< [XX..YY] (flag=ZZ)"); // ok, no - lag
                //info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ)", start as u8);  // ok
                //info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ)", start as u16); // ok
                //info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ)", start as u32); // lag
                // info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FILLERRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRRRRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRRRRRRRRRRRRRRRRRRRRRRRRRR", start as u8); // lag
                // info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FILLERRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRRRRRRRRRRRRRRRRRRRRRRRR", start as u8); // lag
                //info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FILLERRRRRRRRRRRRRRRRRRRRRRRRRRFILLERRR", start as u8); // lag
                // info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FILLERRRRRRRRRRRRR", start as u8); // lag
                // info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FILLER", start as u8); // lag
                // info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ) FI", start as u8); // lag
                //info!("Data transfer <<<<<<<< [{}..YY] (flag=ZZ)", start as u8); // lag

                // info!(
                //     "Data transfer <<<<<<<< [{}..{}] (flag={:x})",
                //     start,
                //     end,
                //     ch_flags::get(),
                // );

                let count = command.read_data(&mut STORAGE[start..end])?;
                STATE.storage_offset += count;

                if STATE.storage_offset == (len * BLOCK_SIZE) as usize {
                    command.pass();
                    STATE.storage_offset = 0;
                }

                info!(
                    "Data transfer <<<<<<<< [{}..{}] (flag={:x})",
                    start,
                    end,
                    ch_flags::get(),
                );

            } else {
                command.pass();
                STATE.storage_offset = 0;
            }
        },
        ScsiCommand::ModeSense6 { .. } => {
            command.try_write_data_all(&[
                0x03, // number of bytes that follow
                0x00, // the media type is SBC
                0x00, // not write-protected, no cache-control bytes support
                0x00, // no mode-parameter block descriptors
            ])?;
            command.pass();
        }
        ScsiCommand::ModeSense10 { .. } => {
            command.try_write_data_all(&[0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?;
            command.pass();
        }
        ref unknown_scsi_kind => {
            error!("Unknown SCSI command: {}", unknown_scsi_kind);
            unsafe {
                STATE.sense_key.replace(0x05); // illegal request Sense Key
                STATE.sense_key_code.replace(0x20); // Invalid command operation ASC
                STATE.sense_qualifier.replace(0x00); // Invalid command operation ASCQ
            }
            command.fail();
        }
    }

    Ok(())
}
