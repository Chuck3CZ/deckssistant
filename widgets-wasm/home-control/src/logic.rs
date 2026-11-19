// Copyright (C) 2026  Braiins Forge s.r.o.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Braiins Systems s.r.o. and Braiins Forge s.r.o. each reserve the right
// to grant any party a license to this program, or any part thereof,
// under any terms, and such a grant shall be considered distinct from
// the grant above.

//! Pure decisions, kept apart from the host so they test without a device.

/// The Home Assistant service a tap on an entity calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tap {
    pub domain: &'static str,
    pub service: &'static str,
}

/// The domain of an entity id: `light` for `light.living_room`.
#[must_use]
pub fn domain_of(entity_id: &str) -> Option<&str> {
    let (domain, object) = entity_id.split_once('.')?;
    (!domain.is_empty() && !object.is_empty()).then_some(domain)
}

/// What a tap does for this entity; `None` for a domain that only displays.
#[must_use]
pub fn tap_for(entity_id: &str) -> Option<Tap> {
    let (domain, service) = match domain_of(entity_id)? {
        "light" => ("light", "toggle"),
        "switch" => ("switch", "toggle"),
        "input_boolean" => ("input_boolean", "toggle"),
        "fan" => ("fan", "toggle"),
        "cover" => ("cover", "toggle"),
        "automation" => ("automation", "toggle"),
        "scene" => ("scene", "turn_on"),
        "script" => ("script", "turn_on"),
        "button" => ("button", "press"),
        "input_button" => ("input_button", "press"),
        _ => return None,
    };
    Some(Tap { domain, service })
}


/// An entity id the widget will put into a request: lowercase letters, digits, `_`
/// and one `.` between a domain and an object id. Anything else is dropped, so an
/// operator's text can never reach the template or a URL as syntax.
#[must_use]
pub fn valid_entity_id(id: &str) -> bool {
    let Some((domain, object)) = id.split_once('.') else {
        return false;
    };
    let ok = |s: &str| {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    ok(domain) && ok(object) && id.len() <= 255
}

/// `http://host:8123/` (or without the scheme-less slash) as a bare base, or `None`.
#[must_use]
pub fn base_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let rest = trimmed.strip_prefix("http://")?;
    let valid = !rest.is_empty()
        && !rest.contains(['/', '?', '#', '@', ' ', '{', '}'])
        && rest.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'_'));
    valid.then(|| trimmed.to_string())
}

fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for c in raw.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// The body of a service call for one entity.
#[must_use]
pub fn service_request(entity_id: &str) -> String {
    format!("{{\"entity_id\":\"{}\"}}", json_escape(entity_id))
}

/// Whether a state string reads as "on" for highlighting.
#[must_use]
pub fn is_active(state: &str) -> bool {
    matches!(state, "on" | "open" | "playing" | "home" | "unlocked")
}

/// The line under a tile's name: the state, with its unit when there is one.
#[must_use]
pub fn state_line(state: &str, unit: Option<&str>) -> String {
    match (state, unit) {
        ("unavailable", _) => "Unavailable".to_string(),
        ("unknown", _) => "Unknown".to_string(),
        (s, Some(u)) if !u.is_empty() => format!("{s} {u}"),
        (s, _) => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use bmc_wasm_sdk::wasm_test;

    use super::*;

    #[wasm_test]
    fn taps_by_domain() {
        assert_eq!(tap_for("light.kitchen").map(|t| t.service), Some("toggle"));
        assert_eq!(tap_for("scene.movie").map(|t| t.service), Some("turn_on"));
        assert_eq!(tap_for("button.doorbell").map(|t| t.service), Some("press"));
        assert_eq!(tap_for("sensor.outdoor"), None);
        assert_eq!(tap_for("nodot"), None);
        assert_eq!(tap_for("light."), None);
    }

    #[wasm_test]
    fn entity_ids_and_urls() {
        assert!(valid_entity_id("light.living_room"));
        assert!(!valid_entity_id("light.x'}}"));
        assert!(!valid_entity_id("nodot"));
        assert_eq!(base_url("http://192.168.1.10:8123/").as_deref(), Some("http://192.168.1.10:8123"));
        assert_eq!(base_url("https://ha.local"), None);
        assert_eq!(base_url("http://a/b"), None);
        assert_eq!(base_url(""), None);
    }

    #[wasm_test]
    fn requests() {
        assert_eq!(service_request("light.a"), "{\"entity_id\":\"light.a\"}");
    }

    #[wasm_test]
    fn state_lines() {
        assert_eq!(state_line("21.5", Some("°C")), "21.5 °C");
        assert_eq!(state_line("on", None), "on");
        assert_eq!(state_line("unavailable", Some("°C")), "Unavailable");
    }
}
