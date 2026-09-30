#!/usr/bin/env bash
#
# Local preview with walkthrough screenshots and hot reload.
#
# Builds the site, runs the dogfood test once to (re)generate the receipts into
# public/receipts/, then serves with Trunk. Trunk re-copies public/receipts/
# into dist on every rebuild, so the screenshots stay visible through hot
# reload. Edit components and the page updates live; re-run this script when you
# want fresh receipts.
#
# Prereq (once): cargo run -p playwright-rs --features cli -- install chromium
set -euo pipefail

cd "$(dirname "$0")"

# The architecture section, exactly as the deploy writes it: asbuilt (a
# prerequisite, see docs/development.md) plus Node and Graphviz for the
# render. A stale model or a missing tool stops the preview here with
# asbuilt's own message, the same as it would stop the deploy. The tree is
# emptied first (the .gitkeep stays) so the preview starts from what CI's
# fresh checkout has: `asbuilt docs` replaces only what it wrote, and a
# stray file here would otherwise ride into dist/.
find public/architecture -mindepth 1 ! -name .gitkeep -delete
asbuilt docs ../.. -o crates/site/public/architecture
trunk build
cargo test --manifest-path ../site-e2e/Cargo.toml
exec trunk serve --open
