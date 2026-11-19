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
    fn state_lines() {
        assert_eq!(state_line("21.5", Some("°C")), "21.5 °C");
        assert_eq!(state_line("on", None), "on");
        assert_eq!(state_line("unavailable", Some("°C")), "Unavailable");
    }
}
