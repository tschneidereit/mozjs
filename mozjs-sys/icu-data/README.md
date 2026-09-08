# Minimized ICU data

SpiderMonkey embeds ICU data in two places. This directory holds the minimized
replacements and the scripts that produce them.

## Files

| file | size | contents |
| --- | --- | --- |
| `locales.txt` | | the locale set, read by every generator here |
| `build_icu4c_data.py` | | builds the ICU4C package |
| `icudt78l-min.dat` | 3.10 MB | the ICU4C package, 258 entries |

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
