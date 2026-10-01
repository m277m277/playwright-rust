//! The vendored driver protocol spec (`protocol-spec/`), read by tests that
//! check what the crate sends against what the driver declares. The driver's
//! validator drops a parameter it does not recognize instead of rejecting it,
//! so a key spelled differently from the spec is otherwise silent: the option
//! simply has no effect.

use std::collections::BTreeSet;

macro_rules! spec_file {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol-spec/",
            $name
        ))
    };
}

const MIXINS: &str = spec_file!("mixins.yml");
pub(crate) const BROWSER: &str = spec_file!("browser.yml");
pub(crate) const BROWSER_TYPE: &str = spec_file!("browserType.yml");

/// The parameter names `command` declares in `spec`, its `$mixin` entries
/// expanded from `mixins.yml`. Panics if the command or a mixin is missing,
/// so a rename in the spec fails the test instead of emptying the set.
pub(crate) fn command_parameters(spec: &str, command: &str) -> BTreeSet<String> {
    let mut lines = spec.lines();
    lines
        .by_ref()
        .find(|line| *line == format!("    {command}:"))
        .unwrap_or_else(|| panic!("no command `{command}` in the spec"));
    lines
        .by_ref()
        .find(|line| *line == "      parameters:")
        .unwrap_or_else(|| panic!("command `{command}` declares no parameters"));
    let mut parameters = BTreeSet::new();
    for line in lines {
        if !line.trim().is_empty() && indent(line) <= 6 {
            break;
        }
        if indent(line) != 8 {
            continue;
        }
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        if key.starts_with("$mixin") {
            parameters.extend(mixin_properties(value.trim()));
        } else {
            parameters.insert(key.to_string());
        }
    }
    parameters
}

/// The property names of the top-level mixin `name` in `mixins.yml`.
fn mixin_properties(name: &str) -> BTreeSet<String> {
    let mut lines = MIXINS.lines();
    lines
        .by_ref()
        .find(|line| *line == format!("{name}:"))
        .unwrap_or_else(|| panic!("no mixin `{name}` in mixins.yml"));
    lines
        .take_while(|line| line.trim().is_empty() || indent(line) > 0)
        .filter(|line| indent(line) == 4)
        .filter_map(|line| line.trim().split_once(':').map(|(key, _)| key.to_string()))
        .collect()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The top-level keys of a serialized options value.
pub(crate) fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("options serialize to a JSON object")
        .keys()
        .cloned()
        .collect()
}
