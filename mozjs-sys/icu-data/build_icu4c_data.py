"""Build a minimized ICU4C data package.

Runs ICU's own data builder over the ICU data sources with a locale filter, so
the package is produced rather than edited. That matters: `res_index.res` is
generated from whichever locales survive, ICU resolves parent locales from its
own dependency data, and the resource filters that trim fields inside a locale
apply as they do upstream. Editing a built package can do none of those.

Needs the ICU data sources and the matching ICU tools, neither of which is in
this tree. See README.md.
"""

import argparse
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
MOZJS = os.path.join(HERE, "..", "mozjs")
DATABUILDER = os.path.join(MOZJS, "intl", "icu", "source", "python")
UPSTREAM_FILTER = os.path.join(MOZJS, "intl", "icu", "data_filter.json")
UVERNUM = os.path.join(
    MOZJS, "intl", "icu", "source", "common", "unicode", "uvernum.h"
)
DEFAULT_LOCALES = os.path.join(HERE, "locales.txt")
DEFAULT_OUTPUT = os.path.join(HERE, "icudt78l-min.dat")

# genrb, pkgdata and the rest, invoked by the data builder.
REQUIRED_TOOLS = ["genrb", "gencnval", "icupkg", "makeconv", "pkgdata"]

# SpiderMonkey builds ICU with UCONFIG_NO_COLLATION and UCONFIG_NO_NORMALIZATION
# and uses ICU4X for both, so this data is 412 KB nothing can read.
UNREADABLE_CATEGORIES = ["coll_ucadata", "normalization"]


def read_locales(path):
    """Locale ids from a list file, one per line. `#` starts a comment."""
    locales = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            locale = line.split("#", 1)[0].strip()
            if locale:
                locales.add(locale)
    return locales


def build_filter(upstream, locales):
    """Merge a locale filter into SpiderMonkey's ICU data filter.

    `localeFilter` reaches a `*_tree` category only when that category's
    `featureFilters` entry is `"include"`, so a tree upstream already filters
    is intersected with the locale filter rather than losing one of the two.
    """
    locale_filter = {
        "filterType": "locale",
        "includeChildren": False,
        "includelist": list(locales),
    }
    merged = json.loads(json.dumps(upstream))
    filters = merged.setdefault("featureFilters", {})
    for category, existing in list(filters.items()):
        if not category.endswith("_tree"):
            continue
        if existing == "exclude" or (
            isinstance(existing, dict) and existing.get("filterType") == "exclude"
        ):
            continue
        filters[category] = (
            locale_filter
            if existing == "include"
            else {
                "filterType": "intersection",
                "intersectionOf": [existing, locale_filter],
            }
        )
    for category in UNREADABLE_CATEGORIES:
        filters[category] = {"filterType": "exclude"}
    merged["localeFilter"] = locale_filter
    return merged


class Package:
    """An ICU4C data package: a header and `(name, payload)` entries."""

    def __init__(self, header, entries):
        self.header = header
        self.entries = entries


def read_package(data):
    """Parse a `.dat` package, for checking what a build produced.

    The format is a `UDataInfo` header followed by an entry count, that many
    `(nameOffset, dataOffset)` pairs, a pool of NUL-terminated names, and the
    payloads. Both offset kinds are relative to the start of the table of
    contents. An entry's length is the distance to the next one.
    """
    header_size = struct.unpack_from("<H", data, 0)[0]
    if data[2] != 0xDA or data[3] != 0x27:
        raise ValueError("not an ICU data package: bad magic")
    toc = header_size
    count = struct.unpack_from("<I", data, toc)[0]
    offsets = [struct.unpack_from("<II", data, toc + 4 + 8 * i) for i in range(count)]

    def name_at(offset):
        start = toc + offset
        return data[start : data.index(b"\0", start)].decode("ascii")

    entries = []
    for i, (name_offset, data_offset) in enumerate(offsets):
        end = offsets[i + 1][1] if i + 1 < count else len(data) - toc
        entries.append((name_at(name_offset), data[toc + data_offset : toc + end]))
    return Package(data[:header_size], entries)


def icu_version():
    """The major ICU version of the vendored SpiderMonkey copy."""
    with open(UVERNUM, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#define U_ICU_VERSION_MAJOR_NUM"):
                return line.split()[2]
    raise ValueError(f"no U_ICU_VERSION_MAJOR_NUM in {UVERNUM}")


def check_tools(tool_dir):
    """Fail with something actionable if the tools are absent or mismatched."""
    missing = [t for t in REQUIRED_TOOLS if not os.path.exists(os.path.join(tool_dir, t))]
    if missing:
        raise SystemExit(
            f"missing ICU tools in {tool_dir}: {', '.join(missing)}\n"
            "Homebrew splits them between bin and sbin; see README.md."
        )
    wanted = icu_version()
    # genrb writes its banner to stderr, as `genrb version 56 (ICU version 78.3).`
    finished = subprocess.run(
        [os.path.join(tool_dir, "genrb"), "--version"],
        capture_output=True, text=True, check=False,
    )
    reported = finished.stderr.splitlines()[0] if finished.stderr else ""
    if f"ICU version {wanted}." not in reported:
        raise SystemExit(
            f"the tools in {tool_dir} are not ICU {wanted}: {reported.strip()}\n"
            "The package format is version-specific; install the matching ICU."
        )


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-dir", required=True,
                        help="the `data` directory of the ICU data sources")
    parser.add_argument("--tool-dir", required=True,
                        help="a directory holding genrb, pkgdata and the rest")
    parser.add_argument("--locales", default=DEFAULT_LOCALES)
    parser.add_argument("--upstream-filter", default=UPSTREAM_FILTER)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    check_tools(args.tool_dir)
    with open(args.upstream_filter, encoding="utf-8") as f:
        upstream = json.load(f)
    locales = sorted(read_locales(args.locales))
    merged = build_filter(upstream, locales)
    package_name = f"icudt{icu_version()}l"

    with tempfile.TemporaryDirectory() as work:
        filter_path = os.path.join(work, "data_filter.json")
        with open(filter_path, "w", encoding="utf-8") as f:
            json.dump(merged, f, indent=2)
        out_dir = os.path.join(work, "out")
        tmp_dir = os.path.join(work, "tmp")
        os.makedirs(out_dir)
        os.makedirs(tmp_dir)

        environment = dict(os.environ, PYTHONPATH=os.path.abspath(DATABUILDER))
        subprocess.run(
            [sys.executable, "-m", "icutools.databuilder", "--mode=unix-exec",
             "--src_dir", ".", "--filter_file", filter_path,
             "--tool_dir", os.path.abspath(args.tool_dir),
             "--out_dir", out_dir, "--tmp_dir", tmp_dir],
            cwd=args.data_dir, env=environment, check=True,
        )
        subprocess.run(
            [os.path.join(args.tool_dir, "pkgdata"), "-q", "-m", "common",
             "-p", package_name, "-s", out_dir, "-d", tmp_dir,
             os.path.join(tmp_dir, "icudata.lst")],
            check=True,
        )
        shutil.copyfile(os.path.join(tmp_dir, f"{package_name}.dat"), args.output)

    with open(args.output, "rb") as f:
        package = read_package(f.read())
    print(
        f"{len(package.entries)} entries, "
        f"{os.path.getsize(args.output) / 1048576:.2f} MB -> {args.output}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
