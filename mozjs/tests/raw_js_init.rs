//! `JS_Init` must work on its own, without going through `JSEngine::init`.
//!
//! ICU data is registered at runtime rather than resolved at link time, so
//! `JS_Init` installs it. Without that, anything reaching the engine through
//! the raw JSAPI gets a failing `JS_Init`.

// Under this feature nothing is embedded for `JS_Init` to install, so there is
// nothing to assert here. `icu_external_supplied` covers the supplied path.
#![cfg(not(feature = "external-icu-data"))]

#[test]
fn js_init_succeeds_without_the_rust_wrapper() {
    assert!(unsafe { mozjs::jsapi::JS_Init() }, "JS_Init failed");
}
