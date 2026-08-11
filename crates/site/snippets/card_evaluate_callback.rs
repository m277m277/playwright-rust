// Hand the page a Rust closure; JS calls back into your test.
let sum: i64 = page
    .evaluate_with_callback("async cb => await cb(20, 22)", |args| async move {
        let a = args[0].as_i64().unwrap_or(0);
        let b = args[1].as_i64().unwrap_or(0);
        serde_json::json!(a + b)
    })
    .await?;
assert_eq!(sum, 42);
