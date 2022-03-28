// Pure Cell
// Copyright © 2022 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//
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
//! pure_cell!(cell, (), |state: u32, _args: ()| {
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
//! let amount = 2;
//! let state = pure_cell!(cell, amount, |state: u32, amount: u32| -> u32 {
//!     state += amount;
//!     state
//! });
//! assert_eq!(state, 17);
//! ```

#![no_std]
#![doc(
    html_logo_url = "https://ardaku.github.io/mm/logo.svg",
    html_favicon_url = "https://ardaku.github.io/mm/icon.svg",
    html_root_url = "https://docs.rs/pure_cell"
)]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

use core::{cell::UnsafeCell, mem::ManuallyDrop};

/// A cell type that provides interior mutability via "pure" functions.
#[derive(Debug)]
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
    (
        $pure_cell:expr,
        $input:expr,
        |$state:ident: $ty:ty, $args:ident: $argty:ty| -> $ret:ty $block:block
    ) => ({
        #[inline(always)]
        const fn const_fn(mut $state: $ty, mut $args: $argty) -> ($ty, $ret) {
            let (output, state) = ($block, $state);
            (state, output)
        }
        fn wrapper_fn(
            state: &mut core::mem::ManuallyDrop<$ty>,
            input: $argty,
        ) -> $ret {
            unsafe {
                let (new, out) = const_fn(
                    core::mem::ManuallyDrop::take(state),
                    input,
                );
                *state = core::mem::ManuallyDrop::new(new);
                out
            }
        }
        unsafe {
            $pure_cell.with(move |state| wrapper_fn(state, $input))
        }
    });
    ($pure_cell:expr, $input:expr, |$state:ident: $ty:ty, $args:ident: $argty:ty| $block:block) => (
        $crate::pure_cell!($pure_cell, $input, |$state: $ty, $args: $argty| -> () $block)
    );
}
