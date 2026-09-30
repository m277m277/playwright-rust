#!/usr/bin/env bash
#
# `asbuilt check` surveys the code in memory and diffs the result against the
# committed docs/architecture/model.c4. The model is generated, so the whole
# fix for a red run is `asbuilt survey` and staging what it writes, never an
# edit to model.c4.
#
# The model is a survey's output, so it is only meaningful against one
# asbuilt release: the one CI installs, pinned in the composite action. A
# different local asbuilt can pass here and fail in CI, or the reverse, so
# the hook insists on the pinned release rather than letting that surface as
# a model diff nobody can explain.
set -uo pipefail

action=.github/actions/install-asbuilt/action.yml
# The `default:` key of the `version` input, and nothing past the next input.
pinned=$(awk '
  /^  version:[[:space:]]*$/ { v = 1; next }
  v && /^  [^ ]/ { exit }
  v && /^    default:/ { sub(/^    default:[[:space:]]*/, ""); gsub(/"/, ""); print; exit }
' "$action")
if [ -z "$pinned" ]; then
  echo "could not read the pinned asbuilt version from $action"
  exit 1
fi

install="cargo install asbuilt --version $pinned --locked"
if ! command -v asbuilt >/dev/null 2>&1; then
  echo "asbuilt is not installed."
  echo "  install with: $install"
  exit 1
fi

installed=$(asbuilt --version)
if [ "$installed" != "asbuilt $pinned" ]; then
  echo "$installed is installed, but CI surveys with asbuilt $pinned."
  echo "  install with: $install"
  exit 1
fi

exec asbuilt check
