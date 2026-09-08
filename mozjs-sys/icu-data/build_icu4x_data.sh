#!/bin/sh
# Build the minimized ICU4X data blobs.
#
# The blobs cannot be built with a released icu4x-datagen. The vendored
# mozjs_icu_normalizer requires a NormalizerNfcV2 marker ("normalizer/nfc/v2",
# payload CanonicalCompositionsNew) that exists in no published icu_normalizer,
# and ComposingNormalizer::try_new_nfc_unstable requires it with no fallback to
# NormalizerNfcV1. It comes from the branch this script checks out, which is
# where the baked normalizer_nfc_v2.rs.data in mozjs-extracted-crates came from.
#
# That branch is a work in progress and needs two accommodations, both applied
# below:
#
#   - Its crates use sibling path dependencies, so the checkouts have to be laid
#     out with the exact directory names used here.
#   - provider/source/src/normalizer/mod.rs reads a character-ranking file from a
#     hardcoded path in the author's home directory. The file is unpublished, and
#     the patch below makes the path come from $ICU4X_NFC_RANKINGS and default to
#     an empty ranking. Rankings only order the linear array: the source comment
#     states that the order "can be changed freely here without changing the
#     lookup code or affecting correctness", and absent entries already default
#     to 0. An empty
#     ranking gives correct data with less favourable memory locality for NFC.
#
# Usage: ./build_icu4x_data.sh [workdir]        (default: ./icu4x-build)
set -eu

here=$(cd "$(dirname "$0")" && pwd)
work=${1:-$here/icu4x-build}
mkdir -p "$work"
cd "$work"

# The checkout must be named `icu4x`: its crates refer to the siblings below by
# relative path, and those refer back to `../icu4x/components/collections`.
clone() {
    if [ ! -d "$2" ]; then
        git clone --quiet --depth 1 --branch "$3" "https://github.com/hsivonen/$1" "$2"
    fi
}
clone icu4x      icu4x      nfdsinglemark
clone utf16_iter utf16_iter cptrie
clone utf8_iter  utf8_iter  cptrie
clone write16    write16    main

normalizer=icu4x/provider/source/src/normalizer/mod.rs
if grep -q '/home/hsivonen/Downloads/out.json' "$normalizer"; then
    python3 - "$normalizer" <<'PY'
import sys
path = sys.argv[1]
source = open(path).read()
old = '''                let ranking_json_string = std::fs::read_to_string("/home/hsivonen/Downloads/out.json").unwrap();
                let rankings: HashMap<char, u64> = serde_json::from_str(&ranking_json_string).unwrap();'''
new = '''                let rankings: HashMap<char, u64> = match std::env::var("ICU4X_NFC_RANKINGS") {
                    Ok(path) => serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap(),
                    Err(_) => HashMap::new(),
                };'''
assert old in source, "the ranking-file patch no longer applies; the branch has moved"
open(path, "w").write(source.replace(old, new, 1))
PY
    echo "patched the hardcoded ranking path"
fi

cargo install --quiet --path icu4x/provider/icu4x-datagen --root "$work/datagen" --locked
datagen="$work/datagen/bin/icu4x-datagen"

locales=$(sed 's/#.*//' "$here/locales.txt" | tr -d ' \t' | grep . | tr '_' '-' | sort | tr '\n' ' ')
markers=$(grep '^[A-Z]' "$here/icu4x-markers.txt" | grep -v '^SegmenterDictionaryAutoV1$' | tr '\n' ' ')

# shellcheck disable=SC2086
"$datagen" --format blob --markers $markers --locales $locales \
    --out "$here/icu4x.postcard" --overwrite
# shellcheck disable=SC2086
"$datagen" --format blob --markers SegmenterDictionaryAutoV1 --locales $locales \
    --out "$here/icu4x-cjk.postcard" --overwrite

ls -l "$here/icu4x.postcard" "$here/icu4x-cjk.postcard"
