# The asbuilt brand

The mark, the wordmark and their fonts, in one directory a host site can
copy whole. `VERSION` is bumped whenever anything here changes; a copy
records the version it took.

| File | What it is |
|---|---|
| `mark.path` | The mark as SVG path data for a `0 0 24 24` viewBox, drawn in `currentColor`: a telescope sighting at 30 degrees over a graduated arc, the surveyor's instrument behind `asbuilt survey`. |
| `mark.svg` | The same path as a standalone SVG in the rust accent (`#ce422b`), for places that cannot set a color, such as docs.rs and a favicon. |
| `lockup-light.svg`, `lockup-dark.svg` | The mark beside the wordmark, with the glyphs as outlines and the bar placed as `wordmark.css` places it, for places that load no web fonts, such as a GitHub README. Light is rust and ink for a white page; dark is the landing site's colors. |
| `wordmark.css` | The AS/BUILT lockup: the two faces, the accent (`--brand-accent`, rust-300 when unset) and the bar under BUILT placed from the fonts' own metrics. The comment at its top is the spec. |
| `fonts/` | Bungee and Bungee Shade, latin subset, woff2, under the SIL Open Font License (`OFL.txt`). |

## Using it

- The mark: an inline `<svg viewBox="0 0 24 24" fill="currentColor"><path d="…"/></svg>` with the contents of `mark.path`, so it takes the surrounding text color. It holds its shape down to 12 pixels.
- The wordmark: import `wordmark.css`, copy `fonts/` beside the built stylesheet, set `--brand-accent` to the host's accent, and use the markup in the comment at its top. Keep the fonts local: the bar is placed from these fonts' metrics and lands wrong on a fallback face.

In the asbuilt repo, `asbuilt docs` draws the mark before the tree's title in every page's header (`crates/asbuilt-core/src/mark.path`, a copy that `scripts/check-brand-copy.sh` keeps identical), and the landing site uses the mark, the wordmark and `mark.svg` as its favicon.
