/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include "mozilla/intl/ICU4XProvider.h"

namespace {
// Owned for the life of the process. ICU4X borrows the blob rather than copying
// it, so the bytes behind it must outlive this too, which is why the Rust side
// requires them to be `'static`.
const icu4x::capi::DataProvider* gProvider = nullptr;
bool gHasSegmenterDictionary = false;
}  // namespace

namespace mozilla::intl {

const icu4x::capi::DataProvider* ICU4XProvider() { return gProvider; }

bool ICU4XHasSegmenterDictionary() { return gHasSegmenterDictionary; }

}  // namespace mozilla::intl

extern "C" bool SetICU4XData(const uint8_t* data, size_t length,
                             const uint8_t* cjk, size_t cjkLength) {
  if (gProvider) {
    return true;
  }

  auto result = icu4x::capi::icu4x_DataProvider_from_byte_slice_mv1(
      icu4x::diplomat::capi::DiplomatU8View{data, length});
  if (!result.is_ok) {
    return false;
  }
  icu4x::capi::DataProvider* provider = result.ok;

  if (cjk) {
    auto overlay = icu4x::capi::icu4x_DataProvider_from_byte_slice_mv1(
        icu4x::diplomat::capi::DiplomatU8View{cjk, cjkLength});
    if (!overlay.is_ok) {
      icu4x::capi::icu4x_DataProvider_destroy_mv1(provider);
      return false;
    }
    // Merging consumes the overlay, leaving it empty, and leaves the union in
    // `provider`.
    auto merged =
        icu4x::capi::icu4x_DataProvider_fork_by_marker_mv1(provider, overlay.ok);
    icu4x::capi::icu4x_DataProvider_destroy_mv1(overlay.ok);
    if (!merged.is_ok) {
      icu4x::capi::icu4x_DataProvider_destroy_mv1(provider);
      return false;
    }
    gHasSegmenterDictionary = true;
  }

  gProvider = provider;
  return true;
}
