//! The reader-visible strings, from `data/strings.json`: a flat map of keys
//! to text. A number a rule decides is a `{placeholder}` the core fills.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings {
    map: BTreeMap<String, String>,
}

impl Strings {
    pub fn parse(json: &str) -> Result<Strings, String> {
        let map: BTreeMap<String, String> =
            serde_json::from_str(json).map_err(|e| format!("The strings file could not be read: {e}"))?;
        if let Some((k, _)) = map.iter().find(|(_, v)| v.trim().is_empty()) {
            return Err(format!("The strings file has an empty \"{k}\"."));
        }
        Ok(Strings { map })
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// The text for a key; a missing key shows as itself, so it is seen and fixed.
    pub fn get(&self, key: &str) -> String {
        self.map.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// The text with each `{name}` replaced.
    pub fn fill(&self, key: &str, values: &[(&str, String)]) -> String {
        let mut t = self.get(key);
        for (name, v) in values {
            t = t.replace(&format!("{{{name}}}"), v);
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_and_an_empty_string_is_refused() {
        let s = Strings::parse(r#"{"a": "A {fish} at {m} m"}"#).unwrap();
        assert_eq!(
            s.fill("a", &[("fish", "snapper".into()), ("m", "3".into())]),
            "A snapper at 3 m"
        );
        assert_eq!(s.get("missing"), "missing");
        assert_eq!(
            Strings::parse(r#"{"a": " "}"#),
            Err("The strings file has an empty \"a\".".into())
        );
    }

    #[test]
    fn the_shipped_strings_hold_no_bare_number() {
        let s = Strings::parse(include_str!("../../../data/strings.json")).unwrap();
        for (k, v) in &s.map {
            let mut depth = 0;
            for c in v.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    d if d.is_ascii_digit() && depth == 0 => panic!("{k}: a number belongs to the core: {v}"),
                    _ => {}
                }
            }
        }
    }
}
