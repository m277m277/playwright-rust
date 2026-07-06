use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

/// Which snapshot this build is: `"dev"` for the main-HEAD build, or the
/// release version (e.g. `"0.14.0"`). Injected by `build.rs` from `SITE_VERSION`.
const CURRENT_VERSION: &str = env!("SITE_VERSION");

/// `/versions.json`, maintained by the deploy: the newest published release and
/// every published version (newest first).
#[derive(Clone, Debug, Default, Deserialize)]
struct Manifest {
    #[serde(default)]
    latest: String,
    #[serde(default)]
    versions: Vec<String>,
}

fn navigate_to(value: &str) {
    let url = match value {
        "dev" => "/dev/".to_string(),
        v => format!("/v{v}/"),
    };
    let _ = window().location().set_href(&url);
}

/// One `<option>` in the switcher: the value posted on change, the visible
/// label, and whether it is the build currently being viewed.
struct VersionOption {
    value: String,
    label: String,
    selected: bool,
}

/// The switcher's options for the given build and (optional) manifest.
///
/// The current build is always present and selected — even before
/// `/versions.json` loads or if the fetch fails — so a release snapshot never
/// falls back to rendering "dev (main)" as the current version. The manifest
/// only contributes the *other* published releases.
fn version_options(current: &str, manifest: Option<&Manifest>) -> Vec<VersionOption> {
    let is_dev = current == "dev";
    let mut opts = vec![VersionOption {
        value: "dev".to_string(),
        label: "dev (main)".to_string(),
        selected: is_dev,
    }];

    let mut listed_current = is_dev;
    if let Some(m) = manifest {
        for v in &m.versions {
            let selected = v == current;
            listed_current |= selected;
            opts.push(VersionOption {
                value: v.clone(),
                label: format!("v{v}"),
                selected,
            });
        }
    }

    // A release build whose version the manifest didn't list (missing/pending
    // fetch, or a snapshot not yet in the manifest) still needs its own option.
    if !listed_current {
        opts.insert(
            1,
            VersionOption {
                value: current.to_string(),
                label: format!("v{current}"),
                selected: true,
            },
        );
    }

    opts
}

/// Top bar showing the current docs version, a dropdown to switch between the
/// `dev` (main HEAD) build and every published release, and a banner when the
/// viewer is not on the latest stable version.
#[component]
pub fn VersionSwitcher() -> impl IntoView {
    let manifest = RwSignal::new(None::<Manifest>);

    // Fetch the manifest so a snapshot can list versions released after it was
    // built. Failure (e.g. no manifest yet) leaves just the current version.
    spawn_local(async move {
        if let Ok(resp) = gloo_net::http::Request::get("/versions.json").send().await
            && resp.ok()
            && let Ok(m) = resp.json::<Manifest>().await
        {
            manifest.set(Some(m));
        }
    });

    let is_dev = CURRENT_VERSION == "dev";

    // (message, link label, link href) when the viewer should be nudged elsewhere.
    let banner = move || -> Option<(String, String, String)> {
        let m = manifest.get()?;
        if is_dev {
            let (label, href) = if m.latest.is_empty() {
                ("the latest release".to_string(), "/".to_string())
            } else {
                (format!("v{}", m.latest), format!("/v{}/", m.latest))
            };
            Some((
                "Unreleased dev build (main) — APIs may change.".to_string(),
                format!("Go to {label} →"),
                href,
            ))
        } else if !m.latest.is_empty() && CURRENT_VERSION != m.latest {
            Some((
                format!("You're viewing v{CURRENT_VERSION} — a newer release is available."),
                format!("Go to v{} →", m.latest),
                format!("/v{}/", m.latest),
            ))
        } else {
            None
        }
    };

    view! {
        <div class="w-full border-b border-rust-700/30 bg-ink-800/80 text-sm">
            <div class="mx-auto flex max-w-5xl items-center gap-3 px-6 py-2">
                <label for="version-select" class="text-rust-50/60">"Version"</label>
                <select
                    id="version-select"
                    class="rounded border border-rust-700/40 bg-ink-900 px-2 py-1 text-rust-50"
                    on:change=move |ev| navigate_to(&event_target_value(&ev))
                >
                    {move || {
                        version_options(CURRENT_VERSION, manifest.get().as_ref())
                            .into_iter()
                            .map(|o| {
                                view! {
                                    <option value=o.value selected=o.selected>
                                        {o.label}
                                    </option>
                                }
                            })
                            .collect_view()
                    }}
                </select>

                {move || {
                    banner()
                        .map(|(msg, label, href)| {
                            view! {
                                <span class="ml-auto flex items-center gap-2 text-rust-300">
                                    <span>{msg}</span>
                                    <a href=href class="font-semibold underline hover:text-rust-500">
                                        {label}
                                    </a>
                                </span>
                            }
                        })
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(latest: &str, versions: &[&str]) -> Manifest {
        Manifest {
            latest: latest.to_string(),
            versions: versions.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn selected_values(opts: &[VersionOption]) -> Vec<&str> {
        opts.iter()
            .filter(|o| o.selected)
            .map(|o| o.value.as_str())
            .collect()
    }

    #[test]
    fn release_build_without_manifest_still_selects_its_own_version() {
        // Fetch pending or failed: the current release must still be the
        // selected option, never "dev (main)".
        let opts = version_options("0.14.0", None);
        assert_eq!(selected_values(&opts), ["0.14.0"]);
    }

    #[test]
    fn release_build_selects_current_when_manifest_omits_it() {
        let m = manifest("0.15.0", &["0.15.0"]);
        let opts = version_options("0.14.0", Some(&m));
        assert_eq!(selected_values(&opts), ["0.14.0"]);
    }

    #[test]
    fn release_build_with_manifest_lists_each_version_once() {
        let m = manifest("0.15.0", &["0.15.0", "0.14.0"]);
        let opts = version_options("0.14.0", Some(&m));
        assert_eq!(selected_values(&opts), ["0.14.0"]);
        assert_eq!(
            opts.iter().filter(|o| o.value == "0.14.0").count(),
            1,
            "the current version must not be duplicated"
        );
    }

    #[test]
    fn dev_build_selects_dev() {
        let m = manifest("0.14.0", &["0.14.0"]);
        let opts = version_options("dev", Some(&m));
        assert_eq!(selected_values(&opts), ["dev"]);
    }
}
