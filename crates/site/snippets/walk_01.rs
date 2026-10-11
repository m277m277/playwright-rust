let page = context.new_page().await?;
page.goto(url, None).await?;

// Locators auto-wait for the WASM app to mount. No sleeps.
expect(page.locator("#hero-title"))
    .to_have_text("Playwright for Rust")
    .await?;

// Find the Docs button the way an agent would: by the ref an
// ai-mode snapshot gives it, as in `link "Docs" [ref=e5]`.
let hero = page
    .locator("#hero")
    .aria_snapshot(AriaSnapshotOptions::default().mode(AriaSnapshotMode::Ai))
    .await?;
let line = hero.lines().find(|l| l.contains(r#"link "Docs""#)).ok_or("no Docs link")?;
let docs = line.split("[ref=").nth(1).and_then(|r| r.split(']').next()).ok_or("no ref")?;
expect(page.get_by_ref(docs))
    .to_have_attribute("id", "cta-docs")
    .await?;
