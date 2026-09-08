//! The ICU4X data blob, shared by the glue crates.
//!
//! Each glue crate builds its ICU4X objects from one process-wide
//! [`BlobDataProvider`]. `mozjs_sys::icu` installs the blob before the engine
//! starts, so by the time any of them runs the provider is set.

use std::sync::OnceLock;

use icu_provider_blob::BlobDataProvider;

static PROVIDER: OnceLock<BlobDataProvider> = OnceLock::new();

/// Installs the ICU4X data blob.
///
/// The bytes are borrowed rather than copied, which is why the lifetime is
/// `'static`. Returns `Err` if the blob is not a postcard data blob, and
/// `Ok(false)` if one was already installed, in which case this one is
/// ignored.
pub fn set_blob(blob: &'static [u8]) -> Result<bool, icu_provider::DataError> {
    let provider = BlobDataProvider::try_new_from_static_blob(blob)?;
    Ok(PROVIDER.set(provider).is_ok())
}

/// The installed ICU4X data provider.
///
/// Panics if no blob was installed. That cannot happen through
/// `JSEngine::init`, which installs the data or fails before the engine runs.
pub fn provider() -> &'static BlobDataProvider {
    PROVIDER
        .get()
        .expect("no ICU4X data: mozjs_sys::icu::install must run before the engine is used")
}
