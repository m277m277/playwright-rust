use leptos::prelude::*;

const E2E_TEST: &str =
    "https://github.com/padamson/playwright-rust/blob/main/crates/site-e2e/tests/landing_page.rs";

#[component]
pub fn DogfoodBanner() -> impl IntoView {
    view! {
        <section id="dogfood-banner" class="mx-auto max-w-5xl px-6 py-12">
            <div class="rounded-2xl border border-rust-500/40 bg-rust-500/5 p-8 text-center">
                <h2 class="text-2xl font-bold text-rust-300">
                    "Tested by the binding it advertises"
                </h2>
                <p class="mx-auto mt-3 max-w-2xl text-rust-50/80">
                    "This page is a Leptos app built in Rust, and playwright-rs drives it end to "
                    "end in CI. If a feature shown here stops working, the build fails and the page "
                    "does not deploy."
                </p>
                <a
                    href=E2E_TEST
                    class="mt-5 inline-block text-sm font-semibold text-rust-300 underline hover:text-rust-500"
                >
                    "See the test"
                </a>
            </div>
        </section>
    }
}
