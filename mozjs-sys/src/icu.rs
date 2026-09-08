/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Supplying ICU with its data.
//!
//! There are two independent bundles. The ICU4X blob backs normalization,
//! collation, segmentation, and the Unicode properties, and is needed even
//! without the `intl` feature because `String.prototype.normalize` uses it. The
//! ICU4C package backs the rest of the Intl API and is needed only with `intl`;
//! SpiderMonkey links an empty ICU4C package, so the real one has to be
//! registered before anything reads it.
//!
//! By default the bytes come from `icu-data/`. Under the `external-icu-data`
//! feature nothing is embedded and the embedder calls [`set_data`] before
//! initializing the engine.

use std::fmt;
use std::sync::OnceLock;

#[cfg(feature = "intl")]
use std::ffi::c_void;

#[cfg(feature = "intl")]
extern "C" {
    fn SetICUCommonData(data: *const c_void) -> bool;
    /// Builds the process-wide ICU4X provider SpiderMonkey's C++ uses. `cjk` is
    /// merged in when non-null, for the segmenter's Chinese and Japanese word
    /// dictionary.
    fn SetICU4XData(data: *const u8, len: usize, cjk: *const u8, cjk_len: usize) -> bool;
}

/// The alignment ICU requires of a data package.
const ALIGNMENT: usize = 16;

/// Wrapper that gives its contents ICU's required alignment. `include_bytes!`
/// alone guarantees none.
#[repr(C, align(16))]
struct Aligned<T: ?Sized>(T);

#[cfg(all(not(feature = "external-icu-data"), feature = "intl"))]
static EMBEDDED_ICU4C: &Aligned<[u8]> = &Aligned(*include_bytes!("../icu-data/icudt78l-min.dat"));

#[cfg(not(feature = "external-icu-data"))]
static EMBEDDED_ICU4X: &Aligned<[u8]> = &Aligned(*include_bytes!("../icu-data/icu4x.postcard"));

#[cfg(all(not(feature = "external-icu-data"), feature = "segmenter-cjk"))]
static EMBEDDED_ICU4X_CJK: &Aligned<[u8]> =
    &Aligned(*include_bytes!("../icu-data/icu4x-cjk.postcard"));

/// Data supplied by the embedder.
struct Supplied {
    icu4c: &'static [u8],
    icu4x: &'static [u8],
    icu4x_cjk: Option<&'static [u8]>,
}

static SUPPLIED: OnceLock<Supplied> = OnceLock::new();

/// Why ICU data could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// No data was available: the `external-icu-data` feature is on and
    /// [`set_data`] was not called.
    Missing,
    /// The `segmenter-cjk` feature is on but no overlay blob was supplied.
    MissingSegmenterCjk,
    /// The ICU4C package is not 16-byte aligned.
    Misaligned,
    /// ICU rejected the ICU4C package.
    Rejected,
    /// The ICU4X blob is not a postcard data blob.
    RejectedIcu4x,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing => f.write_str(
                "no ICU data: call mozjs_sys::icu::set_data before initializing \
                 the engine, or drop the `external-icu-data` feature",
            ),
            Error::MissingSegmenterCjk => f.write_str(
                "the `segmenter-cjk` feature is on but set_data was given no overlay blob",
            ),
            Error::Misaligned => f.write_str("the ICU4C package is not 16-byte aligned"),
            Error::Rejected => f.write_str("ICU rejected the ICU4C package"),
            Error::RejectedIcu4x => f.write_str("the ICU4X blob is not a postcard data blob"),
        }
    }
}

impl std::error::Error for Error {}

/// Supplies the ICU data to use, in place of the embedded copies.
///
/// `icu4c` is a package produced by `icu-data/build_icu4c_data.py`, `icu4x` a
/// blob produced by `icu-data/build_icu4x_data.sh`, and `icu4x_cjk` that
/// script's overlay blob, which is required when the `segmenter-cjk` feature is
/// on and ignored otherwise.
///
/// `icu4c` must be 16-byte aligned. ICU keeps the pointers, which is why the
/// lifetimes are `'static`: the data is borrowed, never copied.
///
/// Call before initializing the engine. Returns `Err` if called more than once.
pub fn set_data(
    icu4c: &'static [u8],
    icu4x: &'static [u8],
    icu4x_cjk: Option<&'static [u8]>,
) -> Result<(), ()> {
    SUPPLIED
        .set(Supplied {
            icu4c,
            icu4x,
            icu4x_cjk,
        })
        .map_err(|_| ())
}

/// Registers the ICU data with ICU4X and, with the `intl` feature, with ICU4C.
///
/// `JS_Init` does this, so an embedder does not have to. Calling it again is
/// cheap and returns the first result: ICU is told about the data once.
pub fn install() -> Result<(), Error> {
    static INSTALLED: OnceLock<Result<(), Error>> = OnceLock::new();
    *INSTALLED.get_or_init(install_once)
}

/// Called by `JS_Init` in jsapi.cpp, so that every way into the engine gets its
/// ICU data and not only `JSEngine::init`.
///
/// Runs before ICU first reads its data, and after any [`set_data`], which is
/// documented to precede engine initialization.
#[no_mangle]
pub extern "C" fn mozjs_install_icu_data() -> bool {
    install().is_ok()
}

fn install_once() -> Result<(), Error> {
    let supplied = SUPPLIED.get();

    let icu4x = match supplied {
        Some(data) => data.icu4x,
        #[cfg(not(feature = "external-icu-data"))]
        None => &EMBEDDED_ICU4X.0,
        #[cfg(feature = "external-icu-data")]
        None => return Err(Error::Missing),
    };
    let icu4x_cjk = match supplied {
        Some(data) => data.icu4x_cjk,
        #[cfg(all(not(feature = "external-icu-data"), feature = "segmenter-cjk"))]
        None => Some(&EMBEDDED_ICU4X_CJK.0),
        #[cfg(not(all(not(feature = "external-icu-data"), feature = "segmenter-cjk")))]
        None => None,
    };
    if cfg!(feature = "segmenter-cjk") && icu4x_cjk.is_none() {
        return Err(Error::MissingSegmenterCjk);
    }

    mozjs_icu_provider_glue::set_blob(icu4x).map_err(|_| Error::RejectedIcu4x)?;

    #[cfg(feature = "intl")]
    {
        let (cjk_ptr, cjk_len) = match icu4x_cjk {
            Some(blob) => (blob.as_ptr(), blob.len()),
            None => (std::ptr::null(), 0),
        };
        if !unsafe { SetICU4XData(icu4x.as_ptr(), icu4x.len(), cjk_ptr, cjk_len) } {
            return Err(Error::RejectedIcu4x);
        }

        let icu4c = match supplied {
            Some(data) => data.icu4c,
            #[cfg(not(feature = "external-icu-data"))]
            None => &EMBEDDED_ICU4C.0,
            #[cfg(feature = "external-icu-data")]
            None => return Err(Error::Missing),
        };
        if icu4c.as_ptr() as usize % ALIGNMENT != 0 {
            return Err(Error::Misaligned);
        }
        if !unsafe { SetICUCommonData(icu4c.as_ptr().cast()) } {
            return Err(Error::Rejected);
        }
    }

    Ok(())
}
