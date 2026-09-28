#!/usr/bin/env bash
#
# `asbuilt check` surveys the code in memory and diffs the result against the
# committed docs/architecture/model.c4. The model is generated, so the whole
# fix for a red run is `asbuilt survey` and staging what it writes, never an
# edit to model.c4.
#
# asbuilt is an external tool on the git-main channel until it releases 0.1.0,
# so it is not in the workspace and a fresh clone will not have it. Naming the
# install command here beats pre-commit's bare "executable not found".
set -uo pipefail

if ! command -v asbuilt >/dev/null 2>&1; then
  echo "asbuilt is not installed."
  echo "  install with: cargo install --git https://github.com/padamson/asbuilt asbuilt --locked"
  exit 1
fi

exec asbuilt check
