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

/// `set_data` takes `&'static [u8]` and ICU keeps the pointer, so the bytes an
/// embedder supplies have to carry ICU's alignment themselves.
#[repr(C, align(16))]
struct Aligned<T: ?Sized>(T);

static DATA: &Aligned<[u8]> =
    &Aligned(*include_bytes!("../../mozjs-sys/icu-data/icudt78l-min.dat"));

#[test]
fn supplied_data_serves_intl() {
    mozjs::icu::set_data(&DATA.0).expect("set_data was called twice");

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
            "String(new Intl.NumberFormat('de').format(1234.5))",
            rval.handle_mut(),
            options,
        )
        .expect("evaluation failed");
        let string = NonNull::new(rval.get().to_string()).expect("not a string");
        assert_eq!(jsstr_to_string(&context, string), "1.234,5");
    }
}
