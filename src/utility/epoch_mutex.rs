use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;

/// A mutual exclusion primitive for protecting shared data that increments a counter to track
/// epochs.
///
/// # Implementation
///
/// This mutex implementation is based on spinlocks. It is not fair and does not support
/// notification.
#[repr(transparent)]
pub struct RawEpochMutex(AtomicUsize);

impl RawEpochMutex {
    /// A [`RawEpochMutex`] instance that is unlocked.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const UNLOCKED: Self = Self(AtomicUsize::new(0));

    /// Returns whether the [`RawEpochMutex`] is currently locked.
    ///
    /// Note that this function cannot be used to check whether the mutex is locked *by the current
    /// execution context*.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.current_epoch() & 1 == 1
    }

    /// Returns the current epoch number of this mutex.
    #[inline(always)]
    pub fn current_epoch(&self) -> usize {
        self.0.load(Relaxed)
    }

    /// Puts the [`RawEpochMutex`] into a locked state.
    ///
    /// # Blocking Behavior
    ///
    /// This function blocks the current thread until the lock is acquired.
    #[inline]
    pub fn lock(&self) {
        let mut old = self.current_epoch();

        loop {
            match self
                .0
                .compare_exchange_weak(old, old.wrapping_add(1), Acquire, Relaxed)
            {
                Ok(_) => break,
                Err(o) => old = o,
            }

            // Don't attempt a compare-and-swap again until the lock seems available.
            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
    }

    /// Unlocks the mutex.
    ///
    /// # Safety
    ///
    /// The mutex must be locked for the current execution context.
    #[inline(always)]
    pub unsafe fn unlock(&self) {
        self.0.fetch_add(1, Release);
    }
}
