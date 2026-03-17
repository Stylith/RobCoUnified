#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <version-or-tag> [changelog-path]" >&2
  exit 2
fi

version="${1#v}"
changelog="${2:-CHANGELOG.md}"

awk -v version="$version" '
BEGIN {
  header = "## " version " - ";
  found = 0;
  in_section = 0;
}
index($0, header) == 1 {
  found = 1;
  in_section = 1;
}
in_section && /^## / && index($0, header) != 1 {
  exit;
}
in_section {
  print;
}
END {
  if (!found) {
    exit 1;
  }
}
' "$changelog"
