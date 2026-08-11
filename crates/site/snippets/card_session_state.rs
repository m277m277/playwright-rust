// Save an authenticated session: cookies, storage, passkeys.
let state = context
    .storage_state(
        StorageStateOptions::default()
            .credentials(true)
            .indexed_db(true),
    )
    .await?;

// Replay it in a fresh context, passkeys included.
let fresh = browser.new_context().await?;
fresh.set_storage_state(state).await?;
