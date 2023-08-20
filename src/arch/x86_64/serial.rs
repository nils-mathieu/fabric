use core::fmt;
use core::fmt::Write;

use super::instr::{inb, outb};
use crate::log::{Level, LogFn};

const PORT: u16 = 0x3F8;

/// A "token" type proving that the serial port has been initialized.
#[derive(Debug, Clone, Copy)]
pub struct SerialTok(());

impl SerialTok {
    /// Creates a new [`SerialTok`] token from nothing.
    ///
    /// # Safety
    ///
    /// This function must be called *after* the serial port has been initialized using the
    /// [`SerialTok::init`] function.
    #[inline(always)]
    pub const unsafe fn unchecked() -> Self {
        Self(())
    }

    /// Creates a new [`SerialTok`] token by initializing the serial port.
    ///
    /// # Safety
    ///
    /// This function may only be called once.
    pub unsafe fn init() -> Self {
        // See https://wiki.osdev.org/Serial_Ports

        // FIXME:
        //  Check if serial actually exists, return an error if it doesn't.
        //  Check for errors.
        //  The goal is to avoid writing to the serial port if it is faulty or not present.

        // TODO:
        //  Add better comments explaining that is going on here.

        #[allow(clippy::identity_op)]
        unsafe {
            outb(PORT + 1, 0x00);
            outb(PORT + 3, 0x80);
            outb(PORT + 0, 0x03);
            outb(PORT + 1, 0x00);
            outb(PORT + 3, 0x03);
            outb(PORT + 2, 0xC7);
            outb(PORT + 4, 0x1E);
            outb(PORT + 4, 0x0F);
        }

        Self(())
    }

    /// Writes a byte to the serial port.
    ///
    /// # Blocking Behavior
    ///
    /// This function blocks until the serial port is ready to accept a byte.
    #[inline]
    pub fn write_byte(self, byte: u8) {
        unsafe {
            while inb(PORT + 5) & 0x20 == 0 {
                core::hint::spin_loop();
            }

            outb(PORT, byte);
        }
    }

    /// Writes a byte to the serial port.
    ///
    /// # Blocking Behavior
    ///
    /// This function blocks until the serial port is ready to accept more bytes.
    pub fn write_bytes(self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte(byte);
        }
    }

    /// Returns a [`LogFn`] that writes to the serial port.
    pub fn log_fn(self) -> LogFn {
        move |lvl, msg| {
            // SAFETY:
            //  `log_fn` requires a `self`, which ensures that the serial port is already
            //  initialized.
            let mut this = unsafe { Self::unchecked() };

            // Write the log level.
            match lvl {
                Level::Trace => this.write_bytes(b"  \x1B[90mTRACE "),
                Level::Info => this.write_bytes(b"   \x1B[34mINFO\x1B[0m "),
                Level::Warn => this.write_bytes(b"   \x1B[33mWARN "),
                Level::Error => this.write_bytes(b"  \x1B[31mERROR "),
            }

            let _ = this.write_fmt(msg);

            match lvl {
                Level::Trace => this.write_bytes(b"\x1B[0m\n"),
                Level::Info => this.write_byte(b'\n'),
                Level::Warn => this.write_bytes(b"\x1B[0m\n"),
                Level::Error => this.write_bytes(b"\x1B[0m\n"),
            }
        }
    }
}

impl Write for SerialTok {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}
