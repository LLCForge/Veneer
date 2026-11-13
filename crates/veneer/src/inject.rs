use crate::embedded::{COLLECT_PARTS, INJECT_PARTS};
use crate::model::Profile;
use crate::Error;

/// Joins script modules in file order. `00-*` modules share their top-level names (helpers);
/// every other module runs in its own block so its constants cannot clash with another module's.
pub(crate) fn assemble(parts: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (name, code) in parts {
        if name.starts_with("00-") {
            out.push_str(code);
        } else {
            out.push_str(&format!("// {name}\n{{\n{code}}}\n"));
        }
    }
    out
}

/// JavaScript to run before any page script (`addInitScript` / `evaluateOnNewDocument`).
///
/// Profile data is embedded only as `serde_json` output, so no profile value can escape its string literal.
pub fn to_init_script(profile: &Profile) -> Result<String, Error> {
    let json = serde_json::to_string(profile)?;
    Ok(format!(
        "(() => {{\n'use strict';\nconst P = {json};\n{}}})();\n",
        assemble(INJECT_PARTS)
    ))
}

/// Expression for `page.evaluate` that resolves to the values a page can observe.
/// Collector modules add fields to `out`; the profile is not involved.
pub fn collector_script() -> String {
    format!(
        "(async () => {{\n{}return out;\n}})()",
        assemble(COLLECT_PARTS)
    )
}
