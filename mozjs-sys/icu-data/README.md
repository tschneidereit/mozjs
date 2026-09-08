# Minimized ICU data

SpiderMonkey embeds ICU data in two places. This directory holds the minimized
replacements and the scripts that produce them.

## Files

| file | size | contents |
| --- | --- | --- |
| `locales.txt` | | the locale set, read by every generator here |
| `build_icu4c_data.py` | | builds the ICU4C package |
| `icudt78l-min.dat` | 3.10 MB | the ICU4C package, 258 entries |
| `build_icu4x_data.sh` | | builds the ICU4X blobs |
| `icu4x-markers.txt` | | the ICU4X markers, with how they were derived |
| `icu4x.postcard` | 1.91 MB | ICU4X data, LSTM segmenter |
| `icu4x-cjk.postcard` | 1.91 MB | the Chinese and Japanese word dictionary |

## Regenerating

The package is built by ICU's own data builder rather than edited afterwards,
which needs two things this tree does not carry.

1. The ICU data sources, matching the vendored ICU version (78.3):

   ```sh
   gh release download release-78.3 --repo unicode-org/icu \
       --pattern 'icu4c-78.3-data.zip'
   unzip icu4c-78.3-data.zip -d icu-data-src   # gives icu-data-src/data
   ```

2. The ICU 78.3 command line tools. On macOS, Homebrew splits them across two
   directories, and the builder wants one, so link them together:

   ```sh
   brew install icu4c@78
   mkdir icu-tools
   ln -s "$(brew --prefix icu4c@78)"/bin/* "$(brew --prefix icu4c@78)"/sbin/* icu-tools/
   ```

Then:

```sh
python3 build_icu4c_data.py --data-dir icu-data-src/data --tool-dir icu-tools
python3 -m unittest test_build_icu4c_data
```

The version of the tools must match the vendored ICU, because the package
format is version-specific; the script checks this and stops if it does not.
So an update that bumps SpiderMonkey's ICU needs the matching data archive and
tools.

### The ICU4X blobs

```sh
./build_icu4x_data.sh                 # or: ./build_icu4x_data.sh /path/to/workdir
```

The script clones what it needs, builds a datagen, and writes both blobs. It is
idempotent and reproduces them byte for byte, so re-running it is the way to
check a change to `locales.txt` or `icu4x-markers.txt`. Everything below is what
it does and why, for whoever has to repeat it against a newer ICU4X.

A released `icu4x-datagen` cannot build these blobs. The vendored
`mozjs_icu_normalizer` requires a `NormalizerNfcV2` marker
(`"normalizer/nfc/v2"`, payload `CanonicalCompositionsNew`) that exists in no
published `icu_normalizer` (checked 2.1.1, 2.2.0 and 2.3.0), and
`ComposingNormalizer::try_new_nfc_unstable` requires it with no fallback to
`NormalizerNfcV1`. It comes from
[hsivonen/icu4x@nfdsinglemark](https://github.com/hsivonen/icu4x/tree/nfdsinglemark),
which is where the baked `normalizer_nfc_v2.rs.data` in `mozjs-extracted-crates`
came from.

That branch is a work in progress, and building its datagen needs two things:

1. **Exact directory names.** Its crates use sibling path dependencies. The
   checkout must be named `icu4x`, with these as siblings of it:

   | repository | branch | why |
   | --- | --- | --- |
   | `hsivonen/icu4x` | `nfdsinglemark` | defines `NormalizerNfcV2` |
   | `hsivonen/utf16_iter` | `cptrie` | `main` has no `icu_collections` feature |
   | `hsivonen/utf8_iter` | `cptrie` | same |
   | `hsivonen/write16` | `main` | |

   The siblings refer back to `../icu4x/components/collections`, so a checkout
   named anything else fails to resolve.

2. **One patch.** `provider/source/src/normalizer/mod.rs` reads a
   character-ranking file from `/home/hsivonen/Downloads/out.json`, which is not
   published. The script rewrites that to read `$ICU4X_NFC_RANKINGS` and to
   default to an empty ranking.

   An empty ranking is correct. The rankings only order the `linear` array, and
   the source comment states that the order "can be changed freely here without
   changing the lookup code or affecting correctness". Absent entries already
   default to `0`.
   What is lost is memory locality for NFC lookups, not correctness. Set
   `ICU4X_NFC_RANKINGS` to a JSON object mapping characters to frequency ranks
   if you have one.

The two blobs are split so that the Chinese and Japanese word dictionary, which
is half the ICU4X data, is only present when the `segmenter-cjk` feature asks for
it. The runtime merges them with `fork_by_marker`.

**This is not a reproducible toolchain the way the ICU4C side is.** It depends on
a personal work-in-progress branch and a patch to a hardcoded path. An ICU4X
update means checking whether the branch still applies, whether `NormalizerNfcV2`
has landed upstream, or whether the normalizer can move to `NormalizerNfcV1`.

The data is also exercised through the JS API. `mozjs/tests/icu_data.rs` covers
the embedded path; the two `icu_external_*` tests cover the opt-out:

```sh
cargo test -p mozjs --test icu_data
cargo test -p mozjs --features external-icu-data --test icu_external_missing
cargo test -p mozjs --features external-icu-data --test icu_external_supplied
```

## What the filter does

`build_icu4c_data.py` merges a locale filter into SpiderMonkey's own
`intl/icu/data_filter.json` rather than replacing it, so the resource filters
that trim fields inside each locale still apply. It then drops two categories
that SpiderMonkey cannot read at all, 412 KB in total: `coll_ucadata` and
`normalization`, which `defs.mozbuild` disables with `UCONFIG_NO_COLLATION` and
`UCONFIG_NO_NORMALIZATION` because ICU4X does both.

One subtlety, encoded in a test: `localeFilter` reaches a `*_tree` category only
when that category's `featureFilters` entry is `"include"`. SpiderMonkey gives
`locales_tree` and `rbnf_tree` explicit filters, which would otherwise shadow
it, so those are intersected with the locale filter instead of replaced.

Building the package rather than editing one matters for three things that are
easy to get wrong by hand. `res_index.res`, which lists the locales installed in
a tree, is generated from whatever survives, so `supportedLocalesOf` reports
what the package actually holds. Parent locales come from ICU's own dependency
data, so `en_GB` pulls in `en_001` and not the `en_150` that nothing here
resolves through. And the converter alias table is kept, which matters because
`u_init` reads it through `ucnv_io_countKnownConverters` as its check that any
ICU data is present.

## Behaviour changes

A locale outside `locales.txt` is not in the package, so `Intl` negotiates it
the way ECMA-402 says: `supportedLocalesOf` does not name it, and
`resolvedOptions().locale` reports the locale actually used rather than the one
requested. This differs from upstream servo/mozjs, which ships every locale CLDR
provides.
