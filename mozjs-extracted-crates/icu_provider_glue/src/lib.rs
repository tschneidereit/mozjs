//! The ICU4X data blob, shared by the glue crates.
//!
//! Each glue crate builds its ICU4X objects from one process-wide provider
//! over a [`BlobDataProvider`]. `mozjs_sys::icu` installs the blob before the
//! engine starts, so by the time any of them runs the provider is set.

use std::sync::OnceLock;

use icu_locale::LocaleFallbacker;
use icu_provider_adapters::fallback::LocaleFallbackProvider;
use icu_provider_blob::BlobDataProvider;

static PROVIDER: OnceLock<LocaleFallbackProvider<BlobDataProvider>> = OnceLock::new();

/// Installs the ICU4X data blob.
///
/// The bytes are borrowed rather than copied, which is why the lifetime is
/// `'static`. Returns `Err` if the blob is not a postcard data blob or lacks
/// the locale fallback data, and `Ok(false)` if one was already installed, in
/// which case this one is ignored.
pub fn set_blob(blob: &'static [u8]) -> Result<bool, icu_provider::DataError> {
    let blob = BlobDataProvider::try_new_from_static_blob(blob)?;
    let fallbacker = LocaleFallbacker::try_new_with_buffer_provider(&blob)?;
    Ok(PROVIDER
        .set(LocaleFallbackProvider::new(blob, fallbacker))
        .is_ok())
}

/// The installed ICU4X data provider.
///
/// A request for a locale the blob does not hold returns the data of the
/// nearest ancestor it does hold, down to the root locale. The blob is
/// generated with its locales deduplicated against their parents, so `en-US`,
/// for example, is only found this way.
///
/// Panics if no blob was installed. That cannot happen through
/// `JSEngine::init`, which installs the data or fails before the engine runs.
pub fn provider() -> &'static LocaleFallbackProvider<BlobDataProvider> {
    PROVIDER
        .get()
        .expect("no ICU4X data: mozjs_sys::icu::install must run before the engine is used")
}

/// The installed ICU4X data blob, without locale fallback, for enumerating the
/// data identifiers it holds.
///
/// Panics under the same condition as [`provider`].
pub fn blob() -> &'static BlobDataProvider {
    provider().inner()
}
