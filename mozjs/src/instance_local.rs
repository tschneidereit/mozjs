/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! State scoped to a wasm instance rather than to a thread.
//!
//! Under the component model a task is a thread, and calling an async export spawns one, so on
//! wasm32-wasip3 every export call into an instance starts with fresh thread-local storage. State
//! the engine keeps for the life of a `JSContext` outlives any one call, so a `thread_local!` there
//! would be registered by one call and missing from the next. Linear memory has instance lifetime,
//! and a plain `static` has it too.
//!
//! `mfbt/ThreadLocal.h` makes the same substitution for the C++ side, but excludes a wasi-libc
//! built with cooperative threads, where each of those threads does need its own storage. rustc
//! exposes no cfg for that build of the sysroot, so this branch cannot make the same exclusion.

/// A `static` standing in for a `thread_local!` on wasm32-wasip3.
#[cfg(all(target_family = "wasm", target_env = "p3"))]
pub struct InstanceLocal<T> {
    value: T,
}

// SAFETY: a wasm instance runs guest code on one thread. Concurrent export calls interleave at
// await points rather than running in parallel.
#[cfg(all(target_family = "wasm", target_env = "p3"))]
unsafe impl<T> Sync for InstanceLocal<T> {}

#[cfg(all(target_family = "wasm", target_env = "p3"))]
impl<T> InstanceLocal<T> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Run `f` against the value, matching `LocalKey::with`.
    pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value)
    }
}

/// Declare state scoped to one `JSContext`.
///
/// Takes the shape of a `thread_local!` with a `const` initializer, and expands to an
/// [`InstanceLocal`] static on wasm32-wasip3 and a `thread_local!` everywhere else. Call sites use
/// `.with(|value| ...)` either way.
macro_rules! instance_local {
    ($(
        $(#[$attr:meta])*
        static $name:ident: $ty:ty = $init:expr;
    )*) => {$(
        #[cfg(all(target_family = "wasm", target_env = "p3"))]
        $(#[$attr])*
        static $name: $crate::instance_local::InstanceLocal<$ty> =
            $crate::instance_local::InstanceLocal::new($init);

        #[cfg(not(all(target_family = "wasm", target_env = "p3")))]
        ::std::thread_local! {
            $(#[$attr])*
            static $name: $ty = const { $init };
        }
    )*};
}
