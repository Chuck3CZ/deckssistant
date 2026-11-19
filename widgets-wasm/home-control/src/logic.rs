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

/// How a tile should look, from calm to urgent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Unavailable or unknown.
    Dim,
    /// Idle: off, closed, locked.
    Off,
    /// Running: on, playing, detected.
    On,
    /// Worth a glance: a window or door open, a lock unlocked.
    Warn,
    /// Needs attention: smoke, a leak, tampering.
    Alert,
}

#[must_use]
pub fn tone(entity_id: &str, device_class: Option<&str>, state: &str) -> Tone {
    if matches!(state, "unavailable" | "unknown") {
        return Tone::Dim;
    }
    let domain = domain_of(entity_id).unwrap_or("");
    let class = device_class.unwrap_or("");
    match (domain, state) {
        ("binary_sensor", "on") => match class {
            "smoke" | "moisture" | "tamper" | "gas" | "safety" | "problem" => Tone::Alert,
            "window" | "door" | "opening" | "garage_door" => Tone::Warn,
            _ => Tone::On,
        },
        ("lock", "unlocked" | "open" | "unlocking") => Tone::Warn,
        ("cover", "open" | "opening" | "closing") => Tone::Warn,
        (_, s) if is_active(s) => Tone::On,
        _ => Tone::Off,
    }
}

/// The state as a tile shows it: windows read "Open", not "on".
#[must_use]
pub fn display_state(entity_id: &str, device_class: Option<&str>, state: &str, unit: Option<&str>) -> String {
    let domain = domain_of(entity_id).unwrap_or("");
    let class = device_class.unwrap_or("");
    let word = match (domain, class, state) {
        (_, _, "unavailable") => return "Nedostupné".to_string(),
        (_, _, "unknown") => return "Neznámé".to_string(),
        ("binary_sensor", "window" | "door" | "opening" | "garage_door", "on") => "Otevřeno",
        ("binary_sensor", "window" | "door" | "opening" | "garage_door", "off") => "Zavřeno",
        ("binary_sensor", "motion" | "occupancy" | "presence", "on") => "Pohyb",
        ("binary_sensor", "motion" | "occupancy" | "presence", "off") => "Klid",
        ("binary_sensor", "moisture", "on") => "Mokro",
        ("binary_sensor", "moisture", "off") => "Sucho",
        ("binary_sensor", "smoke", "on") => "Kouř!",
        ("binary_sensor", "smoke", "off") => "V pořádku",
        ("binary_sensor", "tamper", "on") => "Manipulace",
        ("binary_sensor", "tamper", "off") => "V pořádku",
        ("binary_sensor", "vibration", "on") => "Vibrace",
        ("binary_sensor", "vibration", "off") => "Klid",
        ("lock", _, "locked") => "Zamčeno",
        ("lock", _, "unlocked") => "Odemčeno",
        ("cover", _, "open") => "Otevřeno",
        ("cover", _, "closed") => "Zavřeno",
        ("cover", _, "opening") => "Otevírá se",
        ("cover", _, "closing") => "Zavírá se",
        (_, _, "on") if unit.is_none() => "Zapnuto",
        (_, _, "off") if unit.is_none() => "Vypnuto",
        _ => return state_line(state, unit),
    };
    word.to_string()
}

/// The icon for a tile: the entity's own `mdi:` icon when there is one, else a default for its kind.
#[must_use]
pub fn default_icon(entity_id: &str, device_class: Option<&str>, state: &str) -> &'static str {
    let on = is_active(state);
    let class = device_class.unwrap_or("");
    match (domain_of(entity_id).unwrap_or(""), class) {
        ("binary_sensor", "window") => if on { "window-open-variant" } else { "window-closed-variant" },
        ("binary_sensor", "door" | "opening") => if on { "door-open" } else { "door-closed" },
        ("binary_sensor", "garage_door") => if on { "garage-open-variant" } else { "garage-variant" },
        ("binary_sensor", "motion" | "occupancy" | "presence") => if on { "motion-sensor" } else { "motion-sensor-off" },
        ("binary_sensor", "smoke") => "smoke-detector-variant",
        ("binary_sensor", "moisture") => "water-alert",
        ("binary_sensor", "tamper") => "shield-alert",
        ("binary_sensor", "vibration") => "vibrate",
        ("binary_sensor", _) => "alert-circle",
        ("sensor", "temperature") => "thermometer",
        ("sensor", "humidity") => "water-percent",
        ("sensor", "carbon_dioxide") => "molecule-co2",
        ("sensor", "power") => "flash",
        ("sensor", "battery") => "battery",
        ("sensor", _) => "thermometer",
        ("light", _) => if on { "lightbulb" } else { "lightbulb-outline" },
        ("switch", "outlet") => "power-plug",
        ("switch" | "input_boolean", _) => if on { "toggle-switch" } else { "toggle-switch-off-outline" },
        ("lock", _) => if on { "lock-open-variant" } else { "lock" },
        ("cover", "garage") => if on { "garage-open-variant" } else { "garage-variant" },
        ("cover", _) => if on { "blinds-open" } else { "blinds" },
        ("fan", _) => "fan",
        ("climate", _) => "thermostat",
        ("script", _) => "script-text-play",
        ("scene", _) => "movie-open",
        ("button" | "input_button", _) => "gesture-tap-button",
        ("automation", _) => "robot",
        ("media_player", "tv") => "television",
        ("media_player", _) => "speaker",
        ("person", _) => "account",
        _ => "help-circle-outline",
    }
}

/// `mdi:lightbulb` gives `lightbulb`; other icon sets (`ios:…`) give nothing.
#[must_use]
pub fn mdi_name(icon_attr: &str) -> Option<&str> {
    icon_attr.strip_prefix("mdi:").filter(|n| !n.is_empty())
}

/// Grid columns for `count` tiles: bigger tiles while there are few.
#[must_use]
pub fn columns(count: usize) -> usize {
    match count {
        0..=4 => 2.max(count),
        5..=8 => 4,
        9..=15 => 5,
        _ => 6,
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
    fn window_and_door_states() {
        assert_eq!(display_state("binary_sensor.okno", Some("window"), "on", None), "Otevřeno");
        assert_eq!(display_state("binary_sensor.okno", Some("window"), "off", None), "Zavřeno");
        assert_eq!(display_state("binary_sensor.x", Some("opening"), "unavailable", None), "Nedostupné");
        assert_eq!(display_state("lock.dvere", None, "locked", None), "Zamčeno");
        assert_eq!(display_state("sensor.t", Some("temperature"), "21.5", Some("°C")), "21.5 °C");
        assert_eq!(display_state("light.a", None, "on", None), "Zapnuto");
        assert_eq!(tone("binary_sensor.okno", Some("window"), "on"), Tone::Warn);
        assert_eq!(tone("binary_sensor.okno", Some("window"), "off"), Tone::Off);
        assert_eq!(tone("binary_sensor.kour", Some("smoke"), "on"), Tone::Alert);
        assert_eq!(tone("light.a", None, "on"), Tone::On);
        assert_eq!(tone("light.a", None, "unavailable"), Tone::Dim);
    }

    #[wasm_test]
    fn icons_and_grid() {
        assert_eq!(default_icon("binary_sensor.okno", Some("window"), "on"), "window-open-variant");
        assert_eq!(default_icon("light.a", None, "off"), "lightbulb-outline");
        assert_eq!(mdi_name("mdi:lightbulb"), Some("lightbulb"));
        assert_eq!(mdi_name("ios:pc"), None);
        assert_eq!((columns(1), columns(8), columns(9), columns(24)), (2, 4, 5, 6));
    }

    #[wasm_test]
    fn state_lines() {
        assert_eq!(state_line("21.5", Some("°C")), "21.5 °C");
        assert_eq!(state_line("on", None), "on");
        assert_eq!(state_line("unavailable", Some("°C")), "Unavailable");
    }
}
