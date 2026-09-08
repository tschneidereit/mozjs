/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Initializing with `external-icu-data` and no data supplied.
//!
//! Engine initialization is process-wide and happens once, so this is its own
//! test binary rather than a case in `icu_external_supplied`.

#![cfg(all(feature = "intl", feature = "external-icu-data"))]

#[test]
fn without_supplied_data_init_reports_the_missing_package() {
    match mozjs::rust::JSEngine::init() {
        Err(mozjs::rust::JSEngineError::IcuData(mozjs::icu::Error::Missing)) => {}
        Err(other) => panic!("expected IcuData(Missing), got {other:?}"),
        Ok(_) => panic!("expected IcuData(Missing), but the engine initialized"),
    }
}
