/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Checks that the minimized ICU data serves everything that depends on it.
//!
//! Each test names the part of the data it covers, so a filter that drops too
//! much fails here rather than in a downstream embedder.

#![cfg(feature = "intl")]

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

/// Evaluates each expression and returns its result coerced to a string.
///
/// The engine is process-wide and can only be initialized once, so every
/// expression a test file needs goes through a single call.
fn evaluate_all(sources: &[&str]) -> Vec<String> {
    let engine = JSEngine::init().unwrap();
    let mut runtime = Runtime::new(engine.handle());
    let context = runtime.cx();
    let mut results = Vec::with_capacity(sources.len());
    unsafe {
        rooted!(&in(context) let global = JS_NewGlobalObject(
            context,
            &SIMPLE_GLOBAL_CLASS,
            ptr::null_mut(),
            OnNewGlobalHookOption::FireOnNewGlobalHook,
            &*RealmOptions::default(),
        ));
        for source in sources {
            rooted!(&in(context) let mut rval = UndefinedValue());
            let options = CompileOptionsWrapper::new(&context, c"icu_data".to_owned(), 1);
            let script = format!("String({source})");
            evaluate_script(
                context,
                global.handle(),
                &script,
                rval.handle_mut(),
                options,
            )
            .unwrap_or_else(|_| panic!("failed to evaluate {source}"));
            let string = NonNull::new(rval.get().to_string())
                .unwrap_or_else(|| panic!("{source} did not produce a string"));
            results.push(jsstr_to_string(&context, string));
        }
    }
    results
}

#[test]
fn minimized_icu_data_serves_intl_temporal_and_regexp() {
    let results = evaluate_all(&[
        // Locale data for a locale in locales.txt: German groups with '.' and
        // uses ',' as the decimal separator.
        "new Intl.NumberFormat('de').format(1234.5)",
        // en_GB is one of the regional variants, and orders dates day-first.
        "new Intl.DateTimeFormat('en-GB').format(new Date(Date.UTC(2026, 0, 31)))",
        // A locale outside locales.txt falls back to root rather than failing.
        // Danish groups with '.' and uses ',' as the decimal separator, so a
        // root result proves no Danish data was applied. A locale whose own
        // format matches root's would not discriminate.
        "new Intl.NumberFormat('da').format(1234.5)",
        // The default locale depends on the host, so assert only that the
        // request was not matched.
        "new Intl.NumberFormat('da').resolvedOptions().locale !== 'da'",
        // res_index.res is regenerated from the locales that survive
        // filtering, so this reports only locales the package holds.
        "Intl.NumberFormat.supportedLocalesOf(['da', 'cs', 'sv']).join(',')",
        "Intl.NumberFormat.supportedLocalesOf(['de', 'ja', 'en-GB']).join(',')",
        // zoneinfo64.res, which Temporal needs for time zone rules.
        "Temporal.ZonedDateTime.from('2026-03-08T01:30-08:00[America/Los_Angeles]')\
         .add({hours: 1}).offset",
        // uemoji.icu and ulayout.icu, which \\p{...} needs.
        "/^\\p{Emoji_Presentation}$/u.test('\\u{1F600}')",
        "/^\\p{Script=Han}$/u.test('\\u4E2D')",
        // ICU4X collator tailorings.
        "new Intl.Collator('de').compare('a', 'b')",
        // ICU4X segmenter, grapheme granularity.
        "[...new Intl.Segmenter('en', {granularity: 'grapheme'})\
         .segment('a\\u{1F600}b')].length",
        // Currency and display-name resources.
        "new Intl.NumberFormat('de', {style: 'currency', currency: 'EUR'}).format(1)",
        "new Intl.DisplayNames(['de'], {type: 'region'}).of('FR')",
    ]);

    assert_eq!(results[0], "1.234,5", "de number formatting");
    assert_eq!(results[1], "31/01/2026", "en_GB date order");
    assert_eq!(results[2], "1,234.5", "unlisted locale falls back to root");
    assert_eq!(results[3], "true", "the unlisted locale is not matched");
    assert_eq!(
        results[4], "",
        "supportedLocalesOf names no unlisted locale"
    );
    assert_eq!(
        results[5], "de,ja,en-GB",
        "supportedLocalesOf names the listed ones"
    );
    assert_eq!(results[6], "-07:00", "Temporal crossed the DST transition");
    assert_eq!(results[7], "true", "emoji presentation property escape");
    assert_eq!(results[8], "true", "Han script property escape");
    assert_eq!(results[9], "-1", "de collation");
    assert_eq!(results[10], "3", "grapheme segmentation");
    assert_eq!(results[11], "1,00\u{a0}€", "de currency formatting");
    assert_eq!(results[12], "Frankreich", "de region display name");
}
