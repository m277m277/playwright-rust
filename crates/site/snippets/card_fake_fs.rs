let fs = page.fake_file_system().await?;

// Drive the app's "Save As", then assert what it wrote —
// no native picker dialog to click.
page.locator("#save").click(None).await?;
assert_eq!(fs.last_saved_name().await?, Some("plan.json".into()));
