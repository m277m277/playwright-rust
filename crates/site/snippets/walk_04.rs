// Every card shows its own snippet...
for (id, token) in cards {
    expect(page.locator(id)).to_contain_text(token).await?;
}
// ...actually highlighted: one colored span within each card shown.
let shown = page.locator("[id^='feature-']");
let highlighted = page
    .locator("span[style*='color']")
    .first()
    .within(&shown)
    .count()
    .await?;
assert_eq!(highlighted, shown.count().await?);
