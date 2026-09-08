/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::ffi::c_char;

use icu_locale::Direction;
use icu_locale::LanguageIdentifier;
use std::sync::OnceLock;

use icu_locale::LocaleDirectionality;
use icu_locale::LocaleExpander;

/// The locale expander and directionality, each built once from the ICU4X data
/// blob. Building one deserializes from the blob, and this is called per
/// request.
fn expander() -> &'static LocaleExpander {
    static CELL: OnceLock<LocaleExpander> = OnceLock::new();
    CELL.get_or_init(|| {
        LocaleExpander::try_new_extended_with_buffer_provider(icu_provider_glue::provider())
            .expect("the ICU4X data blob has no likely-subtags data")
    })
}

fn directionality() -> &'static LocaleDirectionality {
    static CELL: OnceLock<LocaleDirectionality> = OnceLock::new();
    CELL.get_or_init(|| {
        LocaleDirectionality::try_new_extended_with_buffer_provider(icu_provider_glue::provider())
            .expect("the ICU4X data blob has no script direction data")
    })
}

/// Text direction.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TextDirection {
    /// Unknown text direction.
    Unknown = 0,

    /// Left-to-Right text direction.
    LeftToRight = 1,

    /// Right-to-Left text direction.
    RightToLeft = 2,
}

/// Return the text direction of a language identifier.
///
/// https://tc39.es/ecma402/#sec-textdirectionoflocale
#[no_mangle]
pub unsafe extern "C" fn locale_text_direction_of(
    language_id: *const c_char,
    language_id_len: usize,
) -> TextDirection {
    if language_id_len == 0 {
        return TextDirection::Unknown;
    }

    // Caller should pass LanguageIdentifier, not a Locale!
    let mut lang_id = if let Ok(lang_id) = LanguageIdentifier::try_from_utf8(
        core::slice::from_raw_parts(language_id as *const u8, language_id_len),
    ) {
        lang_id
    } else {
        return TextDirection::Unknown;
    };

    // Manually maximize to handle inputs like "und" or "und-US".
    //
    // https://github.com/unicode-org/icu4x/issues/7866
    if lang_id.script.is_none() {
        expander().maximize(&mut lang_id);

        if lang_id.script.is_none() {
            return TextDirection::Unknown;
        }
    }

    return match directionality().get(&lang_id) {
        Some(Direction::LeftToRight) => TextDirection::LeftToRight,
        Some(Direction::RightToLeft) => TextDirection::RightToLeft,
        Some(_) => TextDirection::Unknown,
        None => TextDirection::Unknown,
    };
}
