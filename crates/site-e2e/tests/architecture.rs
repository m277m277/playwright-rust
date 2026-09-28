//! The architecture section of a deployed snapshot: the output of `asbuilt
//! docs` over this repo's own model, mounted at `architecture/` under the
//! snapshot's base path. Static HTML with relative links, so the assertions
//! are about the tree being complete and its figures resolving under the
//! real base path, which is what a wrong output dir or a missing render
//! would break.
//!
//! Driven by the same `SNAPSHOT_DIST`, `SNAPSHOT_BASE` and `SNAPSHOT_VERSION`
//! variables as the snapshot smoke test, and skips the same way without them.

mod common;

use std::path::{Path, PathBuf};

use playwright_rs::expect;
use playwright_rs::protocol::Page;

use common::{broken_responses, launch_page, serve_snapshot, snapshot_env};

fn architecture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture")
}

/// The ids declared at one indentation level of a `.c4` file whose value
/// starts with `kind`: `  id = container '...' {` for the model's crates,
/// `  view id {` for the curated views. Reading the files keeps this test
/// free of the asbuilt crates: it asserts against what was committed.
fn declared(file: &str, kind: &str) -> Vec<String> {
    let path = architecture_dir().join(file);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let ids: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("  ")?;
            if rest.starts_with(' ') {
                return None;
            }
            if let Some((id, value)) = rest.split_once(" = ") {
                return value.starts_with(kind).then(|| id.to_string());
            }
            // `view server of playwright_rs.server {`: the id is the first
            // word after the keyword.
            let mut words = rest.split_whitespace();
            (words.next()? == kind).then(|| words.next().map(str::to_string))?
        })
        .collect();
    assert!(
        !ids.is_empty(),
        "no `{kind}` declarations in {}",
        path.display()
    );
    ids
}

/// Every `img[src]` under `selector` is one the browser decoded: a real SVG
/// at that path under the base path, not a placeholder and not a 404. The
/// pages mark their images `loading="lazy"`, so `goto` returns before they
/// have started, and one off-screen would never start: each is switched to
/// eager and awaited through `decode()`, which rejects on a broken image.
async fn assert_decoded(page: &Page, selector: &str) {
    let images = page.locator(selector);
    let count = images.count().await.expect("count images");
    assert!(count > 0, "no images under {selector}");
    for i in 0..i32::try_from(count).expect("image count fits i32") {
        let img = images.nth(i);
        let src = img
            .get_attribute("src")
            .await
            .expect("read src")
            .unwrap_or_default();
        let decoded: bool = img
            .evaluate::<bool, ()>(
                "async img => { img.loading = 'eager'; try { await img.decode(); } catch { return false; } return img.naturalWidth > 0; }",
                None,
            )
            .await
            .expect("probe the image");
        assert!(decoded, "`{src}` under {selector} did not decode");
    }
}

fn html_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter(|entry| {
            entry
                .as_ref()
                .is_ok_and(|e| e.path().extension().is_some_and(|ext| ext == "html"))
        })
        .count()
}

#[tokio::test]
async fn snapshot_architecture_lists_every_crate_and_embeds_its_views() {
    let Some((dist, base, _version)) = snapshot_env("architecture test") else {
        return;
    };
    assert!(
        dist.join("architecture/index.html").exists(),
        "SNAPSHOT_DIST has no architecture/index.html: {} (run `asbuilt docs -o crates/site/public/architecture` before the build)",
        dist.display()
    );

    // The committed model is the contract; the generated tree is checked
    // against it. Counting the crate pages catches the parser above going
    // quiet on an emitter change: a shrunken id list would otherwise assert
    // less and stay green.
    let containers = declared("model.c4", "container ");
    let pages = html_files(&dist.join("architecture/containers"));
    assert_eq!(
        containers.len(),
        pages,
        "model.c4 declares {} containers but the tree has {pages} crate pages",
        containers.len()
    );
    let views = declared("views.c4", "view");

    let (addr, server) = serve_snapshot(&dist, &base, None).await;
    let (_pw, browser, page) = launch_page().await;
    let broken = broken_responses(&page).await;

    let root = format!("http://{addr}{base}architecture/");
    page.goto(&root, None)
        .await
        .expect("navigate to the architecture index under the base path");
    expect(page.locator("h1"))
        .to_be_visible()
        .await
        .expect("the architecture index renders");
    for id in &containers {
        let count = page
            .locator(format!("a[href='containers/{id}.html']"))
            .count()
            .await
            .expect("count container links");
        assert!(
            count >= 1,
            "the index does not link crate `{id}` (containers/{id}.html)"
        );
    }
    assert_decoded(&page, "main img[src='views/index.svg']").await;

    // The curated views are the pages the deploy exists to publish; each
    // one named in views.c4 is embedded and decodes.
    page.goto(&format!("{root}views.html"), None)
        .await
        .expect("navigate to the curated views");
    for view in &views {
        assert_decoded(&page, &format!("main img[src='views/{view}.svg']")).await;
    }

    // A crate page: its scoped view is an SVG the browser decoded, not a
    // placeholder from a missing render and not a 404 under the base path.
    let first = &containers[0];
    page.goto(&format!("{root}containers/{first}.html"), None)
        .await
        .expect("navigate to a crate page");
    assert_decoded(&page, &format!("main img[src='../views/view_{first}.svg']")).await;
    let missing = page
        .locator("p.missing")
        .count()
        .await
        .expect("count missing-view placeholders");
    assert_eq!(missing, 0, "the crate page has unrendered views");

    let broken = broken.lock().unwrap().clone();
    assert!(
        broken.is_empty(),
        "responses at or above 400 under {root}:\n{}",
        broken.join("\n")
    );
    browser.close().await.ok();
    server.abort();
}
