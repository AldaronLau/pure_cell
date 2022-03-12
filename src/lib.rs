//! Alternative to `GhostCell` that provides safe interior mutability via const
//! expressions.
//!
//! ## Advantages
//! - Simple, with no cell keys
//! - Works better with thread local global state.
//!
//! ## Disadvantages
//! - Const closures/fn pointers don't exist (yet), so this crate depends on
//!   macro magic to sort-of-polyfill them
//! - Might not always optimize well (TODO)
//!
//! Once const contexts support mutable references, this crate will be able to
//! remove the second disadvantage.  Additionally, once const function pointers
//! stabilize, this crate will be able to remove the first disadvantage.
//!
//! # Getting Started
//! ```rust
//! use pure_cell::{PureCell, pure_cell};
//!
//! let mut cell = PureCell::new(15);
//! pure_cell!(cell, |state: u32| {
//!     state += 1;
//! });
//! let got = cell.get();
//! assert_eq!(*got, 16);
//! ```
//!
//! ```rust
//! use pure_cell::{PureCell, pure_cell};
//!
//! let cell = PureCell::new(15);
//! let state = pure_cell!(cell, |state: u32| -> u32 {
//!     state += 2;
//!     state
//! });
//! assert_eq!(state, 17);
//! ```

use std::{cell::UnsafeCell, mem::ManuallyDrop};

/// A cell type that provides interior mutability via "pure" functions.
pub struct PureCell<T> {
    value: UnsafeCell<ManuallyDrop<T>>,
}

impl<T> PureCell<T> {
    /// Creates a new `PureCell` containing the given value.
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(ManuallyDrop::new(value)),
        }
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Update cell.
    ///
    /// # Safety
    /// Sound to use so long as you follow these rules in the closure:
    ///
    ///  - Must not yield to other code (usually async)
    ///  - Must not recursively call `Self::with()`
    pub unsafe fn with<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut ManuallyDrop<T>) -> R,
    {
        f(&mut *self.value.get())
    }
}

impl<T> Drop for PureCell<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = ManuallyDrop::take(&mut *self.value.get());
        }
    }
}

/// Main safe mechanism to mutate [`PureCell`] via a `const` expression.
#[macro_export]
macro_rules! pure_cell {
    ($pure_cell:expr, |$state:ident: $ty:ty| -> $ret:ty $block:block) => ({
        #[inline(always)]
        const fn const_fn(mut $state: $ty) -> ($ty, $ret) {
            let (output, state) = ($block, $state);
            (state, output)
        }
        fn wrapper_fn(state: &mut core::mem::ManuallyDrop<$ty>) -> $ret {
            unsafe {
                let (new, out) = const_fn(core::mem::ManuallyDrop::take(state));
                *state = core::mem::ManuallyDrop::new(new);
                out
            }
        }
        unsafe {
            $pure_cell.with(wrapper_fn)
        }
    });
    ($pure_cell:expr, |$state:ident: $ty:ty| $block:block) => (
        $crate::pure_cell!($pure_cell, |$state: $ty| -> () $block)
    );
}
