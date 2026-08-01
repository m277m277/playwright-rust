#!/usr/bin/env bash
#
# crates/site and crates/site-e2e are excluded from the workspace (wasm target,
# conflicting features) and carry their own Cargo.lock files, but depend on
# playwright-rs by path. So a workspace dependency change silently staleness
# their lockfiles, and they only catch up whenever someone happens to run a
# cargo command against those manifests — leaving a dirty tree that looks like
# unrelated noise.
#
# This checks rather than rewrites: refreshing a lockfile is a dependency
# resolution change, and it should be an explicit command you ran, not
# something that happened during a commit.
#
# Nothing is broken when this fails — no CI job uses --locked against these
# crates. It is a hygiene gate, so the lockfiles reflect reality and the drift
# does not get rediscovered as a mystery.
set -uo pipefail

status=0
for manifest in crates/site crates/site-e2e; do
  if ! cargo metadata --manifest-path "$manifest/Cargo.toml" --locked --format-version 1 \
       >/dev/null 2>&1; then
    echo "stale lockfile: $manifest/Cargo.lock is behind the workspace."
    echo "  refresh with: cargo metadata --manifest-path $manifest/Cargo.toml >/dev/null"
    echo "  then stage $manifest/Cargo.lock"
    status=1
  fi
done
exit $status
