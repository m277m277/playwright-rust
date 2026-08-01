use leptos::prelude::*;

/// Small "Unreleased" pill marking a dev-only feature.
#[component]
pub fn UnreleasedBadge() -> impl IntoView {
    view! {
        // data attribute, not the text: the dev banner also says "Unreleased",
        // so a text selector would false-match it. The release-snapshot gate
        // asserts zero of these render.
        <span
            data-unreleased-badge
            class="rounded-full border border-rust-500/50 bg-rust-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-rust-300"
        >
            "Unreleased"
        </span>
    }
}
