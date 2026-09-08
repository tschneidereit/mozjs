/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use core::ffi::c_char;
use core::ffi::c_void;
use core::ffi::CStr;
use std::sync::OnceLock;

use icu_collections::codepointinvlist::CodePointInversionListBuilder;

/// The case mapper, built once from the ICU4X data blob.
///
/// Building it per call would deserialize the data every time. The borrowed
/// view taken from it is a struct of references, so taking one per call is
/// inexpensive.
static CASE_MAPPER: OnceLock<icu_casemap::CaseMapper> = OnceLock::new();

fn case_mapper() -> icu_casemap::CaseMapperBorrowed<'static> {
    CASE_MAPPER
        .get_or_init(|| {
            icu_casemap::CaseMapper::try_new_with_buffer_provider(icu_provider_glue::provider())
                .expect("the ICU4X data blob has no case mapping data")
        })
        .as_borrowed()
}

extern "C" {
    fn js_irregexp_add_range_to_zone_list(
        list: *mut c_void,
        zone: *mut c_void,
        start: u32,
        inclusive_end: u32,
    );
}

#[no_mangle]
pub unsafe extern "C" fn mozilla_properties_glue_add_property_ranges(
    list: *mut c_void,
    zone: *mut c_void,
    name: *const c_char,
    negate: bool,
    needs_case_folding: bool,
) -> bool {
    let name = CStr::from_ptr(name).to_bytes();
    // For now, let's use ICU4X to handle only what we no longer
    // have ICU4C support for.
    match name {
        b"Changes_When_NFKC_Casefolded" | b"CWKCF" | b"Changes_When_Casefolded" | b"CWCF" => {}
        _ => {
            return false;
        }
    }
    let Some(Ok(prop)) = icu_properties::CodePointSetData::try_new_for_ecma262_with_buffer_provider(
        icu_provider_glue::provider(),
        name,
    ) else {
        return false;
    };
    let prop = prop.as_borrowed();
    if needs_case_folding {
        let mut builder = CodePointInversionListBuilder::new();
        let mapper = case_mapper();
        for range in prop.iter_ranges() {
            builder.add_range32(range.clone());
            for u in range {
                if let Some(c) = char::from_u32(u) {
                    mapper.add_case_closure_to(c, &mut builder);
                }
            }
        }
        if negate {
            builder.complement();
        }
        let set = builder.build();
        for range in set.iter_ranges() {
            js_irregexp_add_range_to_zone_list(list, zone, *range.start(), *range.end());
        }
    } else if negate {
        for range in prop.iter_ranges_complemented() {
            js_irregexp_add_range_to_zone_list(list, zone, *range.start(), *range.end());
        }
    } else {
        for range in prop.iter_ranges() {
            js_irregexp_add_range_to_zone_list(list, zone, *range.start(), *range.end());
        }
    }
    true
}
