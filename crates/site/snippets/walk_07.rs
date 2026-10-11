// An agent changes the page through a tool the page publishes
// (WebMCP), then the usual locators confirm what it did.
page.webmcp()
    .call_tool::<_, serde_json::Value>(
        "compare_with",
        &serde_json::json!({ "language": ".NET" }),
        None,
    )
    .await?;

expect(page.locator("#comparison [data-lang='.NET']"))
    .to_have_attribute("aria-selected", "true")
    .await?;
