/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Supplying ICU with its data.
//!
//! SpiderMonkey links an empty ICU data package, so the real one has to be
//! registered before anything reads ICU data. By default the bytes come from
//! `icu-data/`. Under the `external-icu-data` feature nothing is embedded and
//! the embedder calls [`set_data`] before initializing the engine.

use std::ffi::c_void;
use std::fmt;
use std::sync::OnceLock;

extern "C" {
    fn SetICUCommonData(data: *const c_void) -> bool;
}

/// The alignment ICU requires of a data package.
const ALIGNMENT: usize = 16;

/// Wrapper that gives its contents ICU's required alignment. `include_bytes!`
/// alone guarantees none.
#[repr(C, align(16))]
struct Aligned<T: ?Sized>(T);

#[cfg(not(feature = "external-icu-data"))]
static EMBEDDED: &Aligned<[u8]> = &Aligned(*include_bytes!("../icu-data/icudt78l-min.dat"));

static SUPPLIED: OnceLock<&'static [u8]> = OnceLock::new();

/// Why ICU data could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// No data was available: the `external-icu-data` feature is on and
    /// [`set_data`] was not called.
    Missing,
    /// The data is not 16-byte aligned.
    Misaligned,
    /// ICU rejected the data as not a valid package.
    Rejected,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing => f.write_str(
                "no ICU data: call mozjs_sys::icu::set_data before initializing \
                 the engine, or drop the `external-icu-data` feature",
            ),
            Error::Misaligned => f.write_str("ICU data is not 16-byte aligned"),
            Error::Rejected => f.write_str("ICU rejected the data package"),
        }
    }
}

impl std::error::Error for Error {}

/// Supplies the ICU data package to use, in place of the embedded one.
///
/// The bytes must be 16-byte aligned and must be a package produced by
/// `icu-data/build_icu4c_data.py`. ICU keeps the pointer, which is why the
/// lifetime is `'static`: the data is borrowed, never copied.
///
/// Call before initializing the engine. Returns the already-supplied data as
/// `Err` if called more than once.
pub fn set_data(icu4c: &'static [u8]) -> Result<(), &'static [u8]> {
    SUPPLIED.set(icu4c)
}

/// Registers the ICU data package with ICU.
///
/// Call once, before anything reads ICU data. The engine initializer does this,
/// so an embedder that goes through it does not call this directly.
pub fn install() -> Result<(), Error> {
    let data = match SUPPLIED.get() {
        Some(data) => *data,
        #[cfg(not(feature = "external-icu-data"))]
        None => &EMBEDDED.0,
        #[cfg(feature = "external-icu-data")]
        None => return Err(Error::Missing),
    };
    if data.as_ptr() as usize % ALIGNMENT != 0 {
        return Err(Error::Misaligned);
    }
    if !unsafe { SetICUCommonData(data.as_ptr().cast()) } {
        return Err(Error::Rejected);
    }
    Ok(())
}
