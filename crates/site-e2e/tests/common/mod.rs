//! Scaffolding shared by the test binaries in this crate: an ephemeral
//! server over a built tree, a fresh Chromium page, and a sink for the
//! responses a page should never see. Each binary uses a subset, hence
//! the dead-code allowance.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use playwright_rs::protocol::{Browser, Page, Playwright};
use tower_http::services::ServeDir;

/// Serve `dist` on an ephemeral port. `overlay` routes are merged ahead of the
/// static fallback, so a test can stub an endpoint the built site fetches (the
/// switcher's `/versions.json`, say) without hand-rolling a second server.
pub async fn serve_with(
    dist: &PathBuf,
    overlay: Option<Router>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = overlay
        .unwrap_or_else(Router::new)
        .fallback_service(ServeDir::new(dist));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind site server");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve site");
    });
    (addr, handle)
}

pub async fn serve(dist: &PathBuf) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    serve_with(dist, None).await
}

/// The snapshot a gate runs against, from `SNAPSHOT_DIST`, `SNAPSHOT_BASE`
/// (the base path it was built for, e.g. `/v0.15.0/`) and `SNAPSHOT_VERSION`
/// (the `SITE_VERSION` used), or `None` to skip when they are unset, so
/// `cargo test` stays useful without a snapshot build.
pub fn snapshot_env(what: &str) -> Option<(PathBuf, String, String)> {
    match (
        std::env::var("SNAPSHOT_DIST"),
        std::env::var("SNAPSHOT_BASE"),
        std::env::var("SNAPSHOT_VERSION"),
    ) {
        (Ok(dist), Ok(base), Ok(version)) => Some((PathBuf::from(dist), base, version)),
        _ => {
            eprintln!("skipping {what}: SNAPSHOT_DIST/BASE/VERSION not set.");
            None
        }
    }
}

/// Serve `dist` under `base` the way gh-pages does, with `overlay` routes
/// (the root `/versions.json`, say) ahead of it. A base of `/` means the
/// root-served layout: no nested mount, `dist` is the fallback itself.
pub async fn serve_snapshot(
    dist: &PathBuf,
    base: &str,
    overlay: Option<Router>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let mount = base.trim_end_matches('/');
    if mount.is_empty() {
        return serve_with(dist, overlay).await;
    }
    let app = overlay
        .unwrap_or_else(Router::new)
        .nest_service(mount, ServeDir::new(dist));
    serve_with(dist, Some(app)).await
}

/// A fresh Chromium page. The `Playwright` and `Browser` handles come back
/// too: dropping either tears down the browser.
pub async fn launch_page() -> (Playwright, Browser, Page) {
    let pw = Playwright::launch().await.expect("launch playwright");
    let browser = pw.chromium().launch().await.expect("launch chromium");
    let page = browser.new_page().await.expect("new page");
    (pw, browser, page)
}

/// Every response at or above 400 the page receives from here on, as
/// `"<status> <url>"`. Registered before navigating, this is the guard for
/// an asset the build pointed at the wrong place under a base path.
pub async fn broken_responses(page: &Page) -> Arc<Mutex<Vec<String>>> {
    let broken: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = broken.clone();
    page.on_response(move |resp| {
        let sink = sink.clone();
        let (status, url) = (resp.status(), resp.url().to_string());
        async move {
            if status >= 400 {
                sink.lock().unwrap().push(format!("{status} {url}"));
            }
            Ok(())
        }
    })
    .await
    .expect("register response listener");
    broken
}
