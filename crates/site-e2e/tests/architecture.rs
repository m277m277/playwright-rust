//! The architecture section of a deployed snapshot: the output of `asbuilt
//! docs` over this repo's own model, mounted at `architecture/` under the
//! snapshot's base path. Static HTML with relative links and every view
//! drawn inline, so the assertions are about the tree being complete, its
//! diagrams drawn from the committed model, its link home and the site's
//! stylesheet resolving under the real base path: what a wrong output dir,
//! a failed render or a stale `[docs]` setting would break.
//!
//! Driven by the same `SNAPSHOT_DIST`, `SNAPSHOT_BASE` and `SNAPSHOT_VERSION`
//! variables as the snapshot smoke test, and skips the same way without them.

mod common;

use std::path::{Path, PathBuf};

use playwright_rs::expect;
use playwright_rs::protocol::{Locator, Page};

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

/// The view LikeC4 drew for `view`, inlined in the page: present, visible,
/// and holding at least one element box, so an empty or placeholder figure
/// fails.
async fn assert_drawn(page: &Page, view: &str) -> Locator {
    let svg = page.locator(format!("main svg.c4[data-view='{view}']"));
    expect(svg.clone())
        .to_be_visible()
        .await
        .unwrap_or_else(|e| panic!("view `{view}` is not drawn on {}: {e:?}", page.url()));
    let nodes = svg
        .locator("g.node")
        .count()
        .await
        .expect("count element boxes");
    assert!(nodes > 0, "view `{view}` draws no elements");
    svg
}

/// How many element boxes of `kind` a drawn view holds.
async fn boxes_of_kind(view: &Locator, kind: &str) -> usize {
    view.locator(format!("g.node.c4-k-{kind}"))
        .count()
        .await
        .expect("count element boxes by kind")
}

/// The edge color the page's stylesheets resolve to, which the site's
/// `architecture.css` sets per scheme.
async fn edge_color(page: &Page) -> String {
    page.evaluate_value(
        "getComputedStyle(document.documentElement).getPropertyValue('--c4-edge').trim()",
    )
    .await
    .expect("read the edge color")
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
async fn snapshot_architecture_lists_every_crate_and_draws_its_views() {
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
    expect(page.locator("meta[name='generator'][content^='asbuilt docs']"))
        .to_have_count(1)
        .await
        .expect("the index is an asbuilt docs page");
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
    let overview = assert_drawn(&page, "index").await;
    let drawn = boxes_of_kind(&overview, "container").await;
    assert_eq!(
        drawn,
        containers.len(),
        "the overview draws {drawn} crates but model.c4 declares {}",
        containers.len()
    );

    // The trail's first crumb goes back to the snapshot the tree ships in
    // (`[docs] home_url`), which only holds if it is relative.
    let home: String = page
        .locator("nav[aria-label='Breadcrumb'] a.home")
        .evaluate::<String, ()>("a => a.href", None)
        .await
        .expect("read the link home");
    assert_eq!(
        home,
        format!("http://{addr}{base}"),
        "the link home leaves the snapshot"
    );

    // The site's palette (`[docs] stylesheet`) is linked after the tree's
    // own and wins: the tree's default edge color is a color-mix, the
    // site's a warm grey, one per scheme. The tree opens dark like the site
    // (`[docs] color_scheme`) and the header's control switches it; a test
    // browser's system scheme is light.
    assert_eq!(
        edge_color(&page).await,
        "#7a655a",
        "the tree does not open in the site's dark palette"
    );
    let scheme = page.locator("#scheme");
    for (choice, expected) in [
        ("light", "#b39a8c"),
        ("system", "#b39a8c"),
        ("dark", "#7a655a"),
    ] {
        scheme
            .select_option(choice, None)
            .await
            .unwrap_or_else(|e| panic!("choose the {choice} scheme: {e:?}"));
        assert_eq!(
            edge_color(&page).await,
            expected,
            "choosing {choice} did not give the site's {choice} palette"
        );
    }

    // The curated views are the pages the deploy exists to publish; each
    // one named in views.c4 is drawn.
    page.goto(&format!("{root}views.html"), None)
        .await
        .expect("navigate to the curated views");
    for view in &views {
        assert_drawn(&page, view).await;
    }

    // A crate page draws its own scoped view.
    let first = &containers[0];
    page.goto(&format!("{root}containers/{first}.html"), None)
        .await
        .expect("navigate to a crate page");
    assert_drawn(&page, &format!("view_{first}")).await;

    let broken = broken.lock().unwrap().clone();
    assert!(
        broken.is_empty(),
        "responses at or above 400 under {root}:\n{}",
        broken.join("\n")
    );
    browser.close().await.ok();
    server.abort();
}
