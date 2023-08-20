use core::fmt;

/// The return value of system calls.
///
/// # Representation
///
/// This is a simple transparent wrapper over a `usize`. Most values should be treated as a simple
/// integer, but the last values are special, as they are used to represent errors.
///
/// Specifically, any value above [`SysResult::FIRST_ERROR`] is an error.
#[derive(Clone, Copy)]
#[repr(transparent)]
#[must_use = "this value represents a system call result, and might represent an error"]
pub struct SysResult(pub usize);

impl SysResult {
    /// The first value that represents an error.
    pub const FIRST_ERROR: usize = -4096isize as usize;

    /// Creates a new [`SysResult`] from the provided value.
    ///
    /// # Panics
    ///
    /// In debug modes, this function panics if the value is above [`SysResult::FIRST_ERROR`].
    ///
    /// This function is not unsafe, and should just be used to sanitize values that are supposed
    /// to be success values.
    #[inline(always)]
    pub const fn success(val: usize) -> SysResult {
        debug_assert!(
            val < Self::FIRST_ERROR,
            "tried to create a success value from an error"
        );
        SysResult(val)
    }

    /// Returns whether this value represents a success.
    #[inline(always)]
    pub const fn is_success(self) -> bool {
        self.0 < Self::FIRST_ERROR
    }

    /// Returns whether this value represents an error.
    #[inline(always)]
    pub const fn is_error(self) -> bool {
        self.0 >= Self::FIRST_ERROR
    }

    /// Returns the value of this [`SysResult`] as an integer.
    ///
    /// # Panics
    ///
    /// If the value is an error, this function panics with the provided message.
    #[track_caller]
    pub const fn expect(self, msg: &str) -> usize {
        if self.is_success() {
            self.0
        } else {
            panic!("{}", msg)
        }
    }

    /// Returns the value of this [`SysResult`] as an integer.
    ///
    /// # Panics
    ///
    /// If the value is an error, this function panics.
    #[track_caller]
    pub const fn unwrap(self) -> usize {
        self.expect("called `SysResult::unwrap()` on an error value")
    }
}

macro_rules! define_error_codes {
    (
        $(
            $(#[$($doc:meta)*])*
            const $name:ident = $value:literal;
        )*
    ) => {
        impl SysResult {
            $(
                $(#[$($doc)*])*
                pub const $name: SysResult = SysResult(Self::FIRST_ERROR + $value);
            )*
        }

        impl fmt::Debug for SysResult {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                if self.is_success() {
                    f.debug_tuple("SysResult")
                        .field(&self.0)
                        .finish()
                } else {
                    match self.0 {
                        $(
                            $value => f.write_str(stringify!($name)),
                        )*
                        _ => f.debug_tuple("SysResult")
                            .field(&format_args!("<unknown error>"))
                            .finish(),
                    }
                }
            }
        }
    };
}

define_error_codes! {
    /// This error is returned when a system call is called with an invalid value.
    ///
    /// Every system call checks all of its input values.
    const INVALID_VALUE = 0;
    /// A process ID passed to a system call was invalid.
    const INVALID_PROCESS_ID = 1;
    /// The system is out of memory and cannot complete the requested operation because of it.
    const OUT_OF_MEMORY = 2;
    /// The requested resource is already used by another process.
    const CONFLICT = 3;
}
