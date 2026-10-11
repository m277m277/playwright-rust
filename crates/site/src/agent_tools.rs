//! The tools this page publishes to AI agents through WebMCP, the experimental
//! browser API that lets a page offer structured tools in place of DOM a model
//! has to read. A browser without it (nearly every one today) gets nothing
//! registered and no difference in the page. The dogfood test lists and calls
//! these from playwright-rs, through `page.webmcp()`.
//!
//! Each tool reads what the page itself shows (the feature cards, the install
//! snippet this build renders), so a tool cannot say something the page does
//! not.

use js_sys::{Function, JSON, Object, Reflect};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// The comparison section's languages, each with the code it shows.
const COMPARISON: [(&str, &str); 3] = [
    ("Python", include_str!("../snippets/example.py")),
    ("Java", include_str!("../snippets/example.java")),
    (".NET", include_str!("../snippets/example.cs")),
];
const COMPARISON_RUST: &str = include_str!("../snippets/example.rs");
/// How long `open_version` waits between answering and navigating. The
/// browser hands a tool's result back to the agent after `execute` returns,
/// not as it returns, so navigating on the next task destroys the page before
/// the answer leaves it; 100 ms is enough locally, and this leaves room for a
/// loaded machine.
const NAVIGATE_AFTER_MS: i32 = 500;

/// Registers the tools when the browser has WebMCP, and does nothing when it
/// does not.
pub fn register() {
    let Some(context) = model_context() else {
        return;
    };
    let Some(register_tool) = Reflect::get(&context, &"registerTool".into())
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok())
    else {
        return;
    };
    let tools = [
        get_install(),
        list_features(),
        get_snippet(),
        compare_with(),
        list_versions(),
        open_version(),
    ];
    for tool in tools {
        // A browser that rejects a tool (a schema it will not take, an API
        // that moved) says why in the console rather than nowhere.
        if let Err(error) = register_tool.call1(&context, &tool) {
            web_sys::console::warn_2(&"WebMCP tool not registered:".into(), &error);
        }
    }
}

/// `document.modelContext`, or `navigator.modelContext` where a browser
/// still puts it there.
fn model_context() -> Option<JsValue> {
    let window = web_sys::window()?;
    let present = |host: &JsValue| {
        Reflect::get(host, &"modelContext".into())
            .ok()
            .filter(|context| !context.is_undefined() && !context.is_null())
    };
    window
        .document()
        .and_then(|document| present(&document.into()))
        .or_else(|| present(&window.navigator().into()))
}

/// Whether a tool only reads the page or also changes what it shows.
#[derive(Clone, Copy, PartialEq)]
enum Effect {
    ReadOnly,
    ChangesThePage,
}

/// A tool definition: what the page tells an agent about the tool, and the
/// function the agent's call runs.
fn tool(
    name: &str,
    description: &str,
    effect: Effect,
    input_schema: Value,
    execute: impl Fn(Value) -> Value + 'static,
) -> JsValue {
    let mut definition = json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    });
    if effect == Effect::ReadOnly {
        definition["annotations"] = json!({ "readOnlyHint": true });
    }
    let definition = js(definition);
    let execute = Closure::<dyn Fn(JsValue) -> JsValue>::new(move |input: JsValue| {
        let input = JSON::stringify(&input)
            .ok()
            .and_then(|text| text.as_string())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        js(execute(input))
    });
    let _ = Reflect::set(&definition, &"execute".into(), &execute.into_js_value());
    definition
}

/// A Rust value as the JavaScript value it describes.
fn js(value: Value) -> JsValue {
    JSON::parse(&value.to_string()).unwrap_or_else(|_| Object::new().into())
}

/// A tool result in the shape agents expect: text content, flagged when it
/// reports a failure rather than an answer.
fn text(text: impl Into<String>, is_error: bool) -> Value {
    let mut result = json!({ "content": [{ "type": "text", "text": text.into() }] });
    if is_error {
        result["isError"] = json!(true);
    }
    result
}

fn get_install() -> JsValue {
    tool(
        "get_install",
        "How to add playwright-rs to a Rust project: the Cargo.toml dependencies \
         for the version this page documents.",
        Effect::ReadOnly,
        json!({ "type": "object", "properties": {} }),
        |_| match query("#install pre").and_then(|block| block.text_content()) {
            Some(block) => text(block.trim(), false),
            None => text("The install section has not rendered.", true),
        },
    )
}

fn list_features() -> JsValue {
    tool(
        "list_features",
        "The playwright-rs features this page shows, each with the id get_snippet \
         takes, its title, and a one-line summary.",
        Effect::ReadOnly,
        json!({ "type": "object", "properties": {} }),
        |_| {
            let features: Vec<Value> = feature_cards()
                .into_iter()
                .map(|card| {
                    json!({
                        "id": card_id(&card),
                        "title": child_text(&card, "h3"),
                        "summary": child_text(&card, "p"),
                    })
                })
                .collect();
            text(Value::Array(features).to_string(), false)
        },
    )
}

fn get_snippet() -> JsValue {
    tool(
        "get_snippet",
        "The Rust code sample one feature card shows (for a card with tabs, the \
         tab it shows now), by an id list_features returns.",
        Effect::ReadOnly,
        json!({
            "type": "object",
            "properties": { "id": { "type": "string", "description": "A feature id, such as \"locators\"." } },
            "required": ["id"],
        }),
        |input| {
            let wanted = input["id"].as_str().unwrap_or_default().to_string();
            let cards = feature_cards();
            match cards.iter().find(|card| card_id(card) == wanted) {
                Some(card) => text(child_text(card, "pre"), false),
                None => {
                    let known: Vec<String> = cards.iter().map(card_id).collect();
                    text(
                        format!("No feature \"{wanted}\". Known ids: {}", known.join(", ")),
                        true,
                    )
                }
            }
        },
    )
}

fn compare_with() -> JsValue {
    let languages: Vec<&str> = COMPARISON.iter().map(|(language, _)| *language).collect();
    tool(
        "compare_with",
        "Show the same browser test in another Playwright language beside its Rust \
         version, the way to port a test to playwright-rs. Switches the page's \
         comparison to that language and returns both versions.",
        Effect::ChangesThePage,
        json!({
            "type": "object",
            "properties": { "language": { "type": "string", "enum": languages } },
            "required": ["language"],
        }),
        |input| {
            let wanted = input["language"].as_str().unwrap_or_default();
            let Some((language, code)) =
                COMPARISON.iter().find(|(language, _)| *language == wanted)
            else {
                return text(
                    format!("No comparison in \"{wanted}\". Try Python, Java, or .NET."),
                    true,
                );
            };
            // The same tab a visitor would click, so the page shows what the
            // agent was given. A tab that is not there is the tool's error,
            // not a success the page does not show.
            let Some(tab) = query(&format!("#comparison [data-lang='{language}']"))
                .and_then(|tab| tab.dyn_into::<web_sys::HtmlElement>().ok())
            else {
                return text(format!("The page has no {language} tab to show."), true);
            };
            tab.click();
            text(
                format!("{language}:\n{code}\nRust:\n{COMPARISON_RUST}"),
                false,
            )
        },
    )
}

fn list_versions() -> JsValue {
    tool(
        "list_versions",
        "The versions of this page that can be opened, as the version switcher \
         lists them, and which one is open.",
        Effect::ReadOnly,
        json!({ "type": "object", "properties": {} }),
        |_| {
            let current = query("#version-select")
                .and_then(|select| select.dyn_into::<web_sys::HtmlSelectElement>().ok())
                .map(|select| select.value())
                .unwrap_or_default();
            let versions: Vec<Value> = version_options()
                .into_iter()
                .map(|(value, label)| json!({ "version": value, "label": label, "open": value == current }))
                .collect();
            text(Value::Array(versions).to_string(), false)
        },
    )
}

fn open_version() -> JsValue {
    tool(
        "open_version",
        "Open this page for another version of playwright-rs, as choosing it in \
         the version switcher does. Takes a version list_versions returns, such \
         as \"0.19.0\" or \"dev\". The page navigates away once it answers.",
        Effect::ChangesThePage,
        json!({
            "type": "object",
            "properties": { "version": { "type": "string" } },
            "required": ["version"],
        }),
        |input| {
            let wanted = input["version"].as_str().unwrap_or_default().to_string();
            let known: Vec<String> = version_options()
                .into_iter()
                .map(|(value, _)| value)
                .collect();
            if !known.contains(&wanted) {
                return text(
                    format!("No version \"{wanted}\". Known: {}", known.join(", ")),
                    true,
                );
            }
            // Answer first, then navigate: leaving the page before the agent
            // has its answer would lose the answer (see NAVIGATE_AFTER_MS).
            let target = wanted.clone();
            let navigate = Closure::once_into_js(move || crate::components::navigate_to(&target));
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    navigate.unchecked_ref(),
                    NAVIGATE_AFTER_MS,
                );
            }
            text(format!("Opening version {wanted}."), false)
        },
    )
}

/// The version switcher's options, as `(value, label)`.
fn version_options() -> Vec<(String, String)> {
    elements("#version-select option")
        .into_iter()
        .map(|option| {
            (
                option.get_attribute("value").unwrap_or_default(),
                option.text_content().unwrap_or_default().trim().to_string(),
            )
        })
        .collect()
}

/// The first element `selector` matches.
fn query(selector: &str) -> Option<web_sys::Element> {
    web_sys::window()?
        .document()?
        .query_selector(selector)
        .ok()
        .flatten()
}

/// Every element `selector` matches.
fn elements(selector: &str) -> Vec<web_sys::Element> {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Vec::new();
    };
    let Ok(nodes) = document.query_selector_all(selector) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|i| nodes.item(i))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .collect()
}

/// The feature cards the page renders right now.
fn feature_cards() -> Vec<web_sys::Element> {
    elements("[id^='feature-']")
}

/// A card's id without the `feature-` prefix every card id carries.
fn card_id(card: &web_sys::Element) -> String {
    card.id().trim_start_matches("feature-").to_string()
}

/// The text of a card's first `selector` element, trimmed.
fn child_text(card: &web_sys::Element, selector: &str) -> String {
    card.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.text_content())
        .unwrap_or_default()
        .trim()
        .to_string()
}
