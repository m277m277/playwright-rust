use playwright_rs::protocol::WaitForFunctionOptions;

#[tokio::test]
async fn test_wait_for_function_resolves_when_page_state_becomes_truthy() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<script>window.ready = false; setTimeout(() => { window.ready = 42; }, 300);</script>",
        None,
    )
    .await
    .expect("set_content");

    let handle = page
        .wait_for_function("() => window.ready", None)
        .await
        .expect("should resolve once window.ready is truthy");

    // The handle carries the expression's value, not just the fact it fired.
    let value: i64 = handle
        .json_value()
        .await
        .expect("read handle value")
        .as_i64()
        .expect("value should be the number assigned");
    assert_eq!(value, 42);

    browser.close().await.expect("close");
}

#[tokio::test]
async fn test_wait_for_function_times_out_when_never_truthy() {
    let (_pw, browser, page) = crate::common::setup().await;
    page.set_content("<script>window.ready = false;</script>", None)
        .await
        .expect("set_content");

    let result = page
        .wait_for_function(
            "() => window.ready",
            WaitForFunctionOptions::default().timeout(1000.0),
        )
        .await;

    assert!(
        result.is_err(),
        "an expression that never becomes truthy should time out"
    );

    browser.close().await.expect("close");
}

#[tokio::test]
async fn test_wait_for_function_polling_interval_sees_off_frame_state() {
    let (_pw, browser, page) = crate::common::setup().await;

    // Exercises the pollingInterval wire path end-to-end on the page-global
    // form (the only form the driver reads it on). This headless page still
    // fires rAF, so this pins acceptance, not that rAF polling would fail.
    page.set_content(
        "<script>window.v = 0; setInterval(() => { window.v++; }, 50);</script>",
        None,
    )
    .await
    .expect("set_content");

    page.wait_for_function(
        "() => window.v > 3",
        WaitForFunctionOptions::default()
            .polling_interval(20.0)
            .timeout(5000.0),
    )
    .await
    .expect("interval polling should observe the counter");

    browser.close().await.expect("close");
}

#[tokio::test]
async fn test_locator_wait_for_function_receives_the_matched_element() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<div id='target' data-state='pending'>x</div>\
         <script>setTimeout(() => \
           document.getElementById('target').dataset.state = 'done', 300);</script>",
        None,
    )
    .await
    .expect("set_content");

    // Bare arrow on purpose: a client-side isFunction guess resolves this
    // vacuously; the elapsed check pins that the wait actually waited.
    let start = std::time::Instant::now();
    page.locator("#target")
        .wait_for_function("el => el.dataset.state === 'done'", None)
        .await
        .expect("should resolve once the bound element reaches the state");
    assert!(
        start.elapsed() >= std::time::Duration::from_millis(250),
        "resolved in {:?}, before the 300ms state flip: the expression cannot have run against the element",
        start.elapsed()
    );

    browser.close().await.expect("close");
}

#[tokio::test]
async fn test_wait_for_function_returns_element_results_as_handles() {
    let (_pw, browser, page) = crate::common::setup().await;

    // The classic wait-until-element-exists form. The driver returns the
    // element as an ElementHandle (a JSHandle subtype on the wire), which
    // must come back usable, not as a type error.
    page.set_content(
        "<script>setTimeout(() => {\
           const d = document.createElement('div');\
           d.className = 'ready';\
           d.textContent = 'here';\
           document.body.appendChild(d);\
         }, 200);</script>",
        None,
    )
    .await
    .expect("set_content");

    let handle = page
        .wait_for_function("() => document.querySelector('.ready')", None)
        .await
        .expect("an element-valued result should resolve to a handle");

    let text = handle
        .get_property("textContent")
        .await
        .expect("read textContent off the element handle")
        .json_value()
        .await
        .expect("json value");
    assert_eq!(text.as_str(), Some("here"));

    browser.close().await.expect("close");
}
