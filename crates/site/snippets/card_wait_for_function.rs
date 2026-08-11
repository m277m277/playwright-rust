// Wait on any page state, not just selectors.
let handle = page
    .wait_for_function("() => window.app?.ready", None)
    .await?;

// Element-scoped: the matched element is the argument.
page.locator("#status")
    .wait_for_function("el => el.dataset.state === 'done'", None)
    .await?;
