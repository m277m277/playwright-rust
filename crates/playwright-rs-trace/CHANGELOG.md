# Changelog — playwright-rs-trace

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the parser can evolve
at its own pace.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Confirmed trace-format v8 compatibility with the Playwright 1.61 driver.**
  The checked-in fixture was regenerated with the 1.61.1 driver (previously a
  1.59.1 trace, predating two driver bumps); 1.61 still emits format version
  8, so the parser is unchanged. The crate docs state the verified driver
  line again, which had been reduced to a bare "trace format v8" while the
  claim was unverified.

## [0.1.0] - 2026-05-23

### Added

- **`TraceReader` — open a Playwright trace zip, stream events, reassemble actions.**
  - `TraceReader::open(reader)` parses the `context-options` event
    eagerly so callers can read `reader.context()` before iterating.
  - `TraceReader::raw_events()` — lossless iterator over every JSONL
    line in `trace.trace`, yielding `RawEvent` (the full JSON object).
    Forward-compat escape hatch for callers dispatching on event kinds
    the parser doesn't model yet.
  - `TraceReader::events()` — typed iterator yielding `TraceEvent`.
    Known kinds become typed variants; anything else surfaces as
    `TraceEvent::Unknown(RawEvent)` so nothing is silently dropped.
  - `TraceReader::actions()` — reassembles `before` + optional `input`
    + zero-or-more `log` + `after` events into a logical `Action`.
    Truncated actions are emitted at end-of-stream rather than
    discarded.
  - Free function `playwright_rs_trace::open(path)` for the
    file-on-disk case.

- **`TraceReader::network()` — `trace.network` parsing → `NetworkEntry` iterator.**
  - One entry per recorded HTTP request/response pair (HAR-like
    resource snapshot). Empty `trace.network` (typical for traces
    driven against `data:` URLs) yields zero items.
  - HAR `-1` "unknown" sentinels are mapped to `None` at parse time —
    the public types use `Option<u64>` / `Option<u16>` / `Option<f64>`
    on `time`, `headers_size`, `body_size`, `status`, `content.size`,
    so callers don't have to know the convention. Empty `redirectURL`
    likewise → `None`.
  - HAR fields not modelled individually (`cookies`, `timings`,
    `cache`, `queryString`, `_transferSize`, …) are preserved on
    `NetworkEntry::raw_snapshot: serde_json::Value`.
  - Unknown event kinds in `trace.network` yield an error rather than
    being silently skipped — the stream is single-purpose.

- **`xtask` workspace member with `regenerate-trace-fixture`
  subcommand.** Drives a real Chromium session through
  `playwright-rs::Tracing` — including a localhost `axum` server so
  the navigation produces a real `resource-snapshot` — to refresh
  the deterministic test fixture under `tests/fixtures/`. New
  `.cargo/config.toml` aliases `cargo xtask`.

[Unreleased]: https://github.com/padamson/playwright-rust/compare/trace-v0.1.0...HEAD
[0.1.0]: https://github.com/padamson/playwright-rust/releases/tag/trace-v0.1.0
