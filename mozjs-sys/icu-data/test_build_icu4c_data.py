"""Tests for the ICU4C data build.

The filter tests are pure. The package tests read the committed artifact, so
they run without the ICU tools and catch a regeneration that went wrong.
"""

import json
import os
import unittest

from build_icu4c_data import build_filter, read_locales, read_package

HERE = os.path.dirname(os.path.abspath(__file__))
LOCALES = os.path.join(HERE, "locales.txt")
UPSTREAM_FILTER = os.path.join(
    HERE, "..", "mozjs", "intl", "icu", "data_filter.json"
)
PACKAGE = os.path.join(HERE, "icudt78l-min.dat")


def upstream():
    with open(UPSTREAM_FILTER, encoding="utf-8") as f:
        return json.load(f)


def package():
    with open(PACKAGE, "rb") as f:
        return read_package(f.read())


class BuildFilter(unittest.TestCase):
    def test_sets_a_locale_filter_from_the_locale_list(self):
        merged = build_filter(upstream(), ["en", "de"])
        self.assertEqual(
            merged["localeFilter"],
            {
                "filterType": "locale",
                "includeChildren": False,
                "includelist": ["en", "de"],
            },
        )

    def test_intersects_a_tree_that_upstream_already_filters(self):
        # `localeFilter` reaches a tree only when its featureFilter is
        # "include", so upstream's own filters have to be composed with it
        # rather than replaced.
        merged = build_filter(upstream(), ["en"])
        self.assertEqual(
            merged["featureFilters"]["locales_tree"]["filterType"], "intersection"
        )
        self.assertIn(
            upstream()["featureFilters"]["locales_tree"],
            merged["featureFilters"]["locales_tree"]["intersectionOf"],
        )

    def test_leaves_an_excluded_tree_excluded(self):
        merged = build_filter(upstream(), ["en"])
        self.assertEqual(
            merged["featureFilters"]["coll_tree"],
            upstream()["featureFilters"]["coll_tree"],
        )

    def test_excludes_what_spidermonkey_builds_icu_without(self):
        # defs.mozbuild sets UCONFIG_NO_COLLATION and UCONFIG_NO_NORMALIZATION,
        # so this data is 412 KB that nothing can read.
        merged = build_filter(upstream(), ["en"])
        for category in ("coll_ucadata", "normalization"):
            self.assertEqual(
                merged["featureFilters"][category], {"filterType": "exclude"}
            )


class CommittedPackage(unittest.TestCase):
    def top_level_locales(self):
        return {
            name.split("/")[-1][: -len(".res")]
            for name, _ in package().entries
            if name.count("/") == 1 and name.endswith(".res")
        }

    def test_holds_every_requested_locale(self):
        present = self.top_level_locales()
        for locale in read_locales(LOCALES):
            self.assertTrue(locale in present, f"{locale} is missing")

    def test_holds_no_locale_that_was_not_requested(self):
        # The `curr` tree holds one file per locale, so its stems name exactly
        # the locales the package holds. `root` and `en_001` are there
        # because ICU resolves requested locales through them.
        locales = {
            name.split("/")[-1][: -len(".res")]
            for name, _ in package().entries
            if name.startswith("icudt78l/curr/") and name.endswith(".res")
        } - {"pool", "res_index", "supplementalData"}
        unexpected = locales - read_locales(LOCALES) - {"root", "en_001"}
        self.assertEqual(unexpected, set(), "unrequested locales are present")

    def test_holds_the_data_regular_expressions_and_temporal_need(self):
        names = {name for name, _ in package().entries}
        for entry in ("uemoji.icu", "ulayout.icu", "zoneinfo64.res",
                      "cnvalias.icu", "res_index.res"):
            self.assertTrue(f"icudt78l/{entry}" in names, f"{entry} is missing")

    def test_stays_within_the_size_budget(self):
        megabytes = os.path.getsize(PACKAGE) / (1024 * 1024)
        self.assertGreater(megabytes, 2.7, f"{megabytes:.2f} MB: over-filtered?")
        self.assertLess(megabytes, 3.4, f"{megabytes:.2f} MB: under-filtered?")


if __name__ == "__main__":
    unittest.main()
