/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
#ifndef intl_components_ICU4XProvider_h_
#define intl_components_ICU4XProvider_h_

#include <stddef.h>
#include <stdint.h>

#include "icu4x/Calendar.hpp"
#include "icu4x/DataProvider.hpp"
#include "icu4x/GraphemeClusterSegmenter.hpp"
#include "icu4x/SentenceSegmenter.hpp"
#include "icu4x/WordSegmenter.hpp"

/**
 * Registers the ICU4X data blob and builds the process-wide provider.
 *
 * `cjk`, when non-null, is merged in and supplies the segmenter's Chinese and
 * Japanese word dictionary. Called from `mozjs_sys::icu` before the engine
 * starts. Returns false if ICU4X rejects either blob. A second call with a
 * provider already registered succeeds and changes nothing.
 */
extern "C" bool SetICU4XData(const uint8_t* data, size_t length,
                             const uint8_t* cjk, size_t cjkLength);

namespace mozilla::intl {

/**
 * The provider registered by `SetICU4XData`, or null if none was.
 */
const icu4x::capi::DataProvider* ICU4XProvider();

/**
 * Whether the registered data includes the Chinese and Japanese word
 * dictionary.
 */
bool ICU4XHasSegmenterDictionary();

}  // namespace mozilla::intl

/**
 * Wrappers carrying the names and signatures of the compiled-data constructors,
 * with the registered provider bound as the leading argument.
 *
 * The call sites name these instead of the `icu4x::capi::` originals, so that
 * the data comes from the blob. They are functions rather than macros because the
 * call sites in Segmenter.cpp take the constructor's address rather than
 * calling it, and because the `_with_provider` variants have a different arity.
 */
namespace mozilla::intl::icu4x_provider {

inline icu4x::capi::GraphemeClusterSegmenter*
icu4x_GraphemeClusterSegmenter_create_mv1() {
  auto result =
      icu4x::capi::icu4x_GraphemeClusterSegmenter_create_with_provider_mv1(
          ICU4XProvider());
  return result.is_ok ? result.ok : nullptr;
}

inline icu4x::capi::icu4x_WordSegmenter_create_auto_with_content_locale_mv1_result
icu4x_WordSegmenter_create_auto_with_content_locale_mv1(
    const icu4x::capi::Locale* locale) {
  icu4x::capi::icu4x_WordSegmenter_create_auto_with_content_locale_mv1_result
      out;
  // Selected from the models the blob holds, so that a build without the
  // dictionary asks for the models it has rather than failing.
  if (ICU4XHasSegmenterDictionary()) {
    auto result = icu4x::capi::
        icu4x_WordSegmenter_create_auto_with_content_locale_and_provider_mv1(
            ICU4XProvider(), locale);
    out.is_ok = result.is_ok;
    if (result.is_ok) {
      out.ok = result.ok;
    } else {
      out.err = result.err;
    }
  } else {
    auto result = icu4x::capi::
        icu4x_WordSegmenter_create_lstm_with_content_locale_and_provider_mv1(
            ICU4XProvider(), locale);
    out.is_ok = result.is_ok;
    if (result.is_ok) {
      out.ok = result.ok;
    } else {
      out.err = result.err;
    }
  }
  return out;
}

inline icu4x::capi::icu4x_SentenceSegmenter_create_with_content_locale_mv1_result
icu4x_SentenceSegmenter_create_with_content_locale_mv1(
    const icu4x::capi::Locale* locale) {
  auto result = icu4x::capi::
      icu4x_SentenceSegmenter_create_with_content_locale_and_provider_mv1(
          ICU4XProvider(), locale);
  icu4x::capi::icu4x_SentenceSegmenter_create_with_content_locale_mv1_result out;
  out.is_ok = result.is_ok;
  if (result.is_ok) {
    out.ok = result.ok;
  } else {
    out.err = result.err;
  }
  return out;
}

inline icu4x::capi::Calendar* icu4x_Calendar_create_mv1(
    icu4x::capi::CalendarKind kind) {
  auto result =
      icu4x::capi::icu4x_Calendar_create_with_provider_mv1(ICU4XProvider(), kind);
  return result.is_ok ? result.ok : nullptr;
}

}  // namespace mozilla::intl::icu4x_provider

#endif /* intl_components_ICU4XProvider_h_ */
