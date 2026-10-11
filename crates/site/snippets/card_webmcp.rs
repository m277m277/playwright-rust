// List the tools the page registered for agents, then call one.
let tools = page.webmcp().tools(None).await?;
let sum: serde_json::Value = page
    .webmcp()
    .call_tool("add", &serde_json::json!({ "a": 2, "b": 40 }), None)
    .await?;
