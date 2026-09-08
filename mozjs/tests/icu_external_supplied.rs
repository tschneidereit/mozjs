/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The embedder-supplied path: no data embedded, `set_data` provides it.

#![cfg(all(feature = "intl", feature = "external-icu-data"))]

use std::ptr;
use std::ptr::NonNull;

use mozjs::conversions::jsstr_to_string;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsval::UndefinedValue;
use mozjs::rooted;
use mozjs::rust::wrappers2::JS_NewGlobalObject;
use mozjs::rust::{
    evaluate_script, CompileOptionsWrapper, JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS,
};

/// ICU keeps the pointer to the ICU4C package and requires it 16-byte aligned,
/// so an embedder has to align the bytes it supplies. The ICU4X blob has no
/// alignment requirement, but goes through the same wrapper here.
#[repr(C, align(16))]
struct Aligned<T: ?Sized>(T);

static ICU4C: &Aligned<[u8]> =
    &Aligned(*include_bytes!("../../mozjs-sys/icu-data/icudt78l-min.dat"));
static ICU4X: &Aligned<[u8]> = &Aligned(*include_bytes!("../../mozjs-sys/icu-data/icu4x.postcard"));

#[test]
fn supplied_data_serves_intl() {
    mozjs::icu::set_data(&ICU4C.0, &ICU4X.0, None).expect("set_data was called twice");

    let engine = JSEngine::init().expect("engine did not accept the supplied data");
    let mut runtime = Runtime::new(engine.handle());
    let context = runtime.cx();
    unsafe {
        rooted!(&in(context) let global = JS_NewGlobalObject(
            context,
            &SIMPLE_GLOBAL_CLASS,
            ptr::null_mut(),
            OnNewGlobalHookOption::FireOnNewGlobalHook,
            &*RealmOptions::default(),
        ));
        rooted!(&in(context) let mut rval = UndefinedValue());
        let options = CompileOptionsWrapper::new(&context, c"icu_external".to_owned(), 1);
        evaluate_script(
            context,
            global.handle(),
            // Covers both bundles: the ICU4C package for the locale data and
            // the ICU4X blob for normalization.
            "String(new Intl.NumberFormat('de').format(1234.5) + '|' + \
             'a\\u0301'.normalize('NFC'))",
            rval.handle_mut(),
            options,
        )
        .expect("evaluation failed");
        let string = NonNull::new(rval.get().to_string()).expect("not a string");
        assert_eq!(jsstr_to_string(&context, string), "1.234,5|\u{e1}");
    }
}
