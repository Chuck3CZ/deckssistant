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

//! The entry points the host calls. One Home Assistant client lives for the
//! widget's lifetime; its subscription is rebuilt when the entity list changes.

use std::cell::RefCell;

use bmc_wasm_sdk::*;
use home_assistant::{Client, ConnectError, Event, Presence, ServiceData, Status, Subscription};

use crate::logic::{is_active, state_line, tap_for};
use crate::manifest_params::Params;

const SLOT: &str = "ha";
const COLUMNS: usize = 4;
const GAP: f32 = 8.0;
const NAME_SIZE: u32 = 16;
const STATE_SIZE: u32 = 22;
const MESSAGE_SIZE: u32 = 22;

struct Ha {
    client: Client,
    sub: Option<Subscription>,
    ids: Vec<String>,
}

thread_local! {
    static HA: RefCell<Option<Ha>> = const { RefCell::new(None) };
}

fn wanted_ids() -> Vec<String> {
    Params::current()
        .entities
        .iter()
        .map(|e| e.entity_id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

/// (Re)subscribe to the configured entities, if the list changed.
fn sync_subscription(ha: &mut Ha) {
    let ids = wanted_ids();
    if ha.sub.is_some() && ids == ha.ids {
        return;
    }
    ha.sub = None;
    if ids.is_empty() {
        ha.ids = ids;
        return;
    }
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    match ha.client.subscribe(&refs, &["friendly_name", "unit_of_measurement"]) {
        Ok(sub) => {
            ha.sub = Some(sub);
            ha.ids = ids;
        }
        Err(_) => log_error!("home-control: Home Assistant subscription refused"),
    }
}

fn on_ha_event(_client: &Client, event: &Event) {
    if matches!(event, Event::StatusChanged(_) | Event::EntityChanged { .. }) {
        request_frame_after(100);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    let client = match home_assistant::connect(SLOT, on_ha_event) {
        Ok(client) => client,
        Err(ConnectError::NoTimerAvailable) => {
            log_error!("home-control: no timer left for the Home Assistant client");
            return;
        }
    };
    let mut ha = Ha { client, sub: None, ids: Vec::new() };
    sync_subscription(&mut ha);
    HA.set(Some(ha));
    request_frame();
}

#[unsafe(no_mangle)]
pub extern "C" fn on_credentials_update() {
    HA.with_borrow(|ha| {
        if let Some(ha) = ha {
            ha.client.resume();
        }
    });
    request_frame();
}

#[unsafe(no_mangle)]
pub extern "C" fn on_params_update() {
    HA.with_borrow_mut(|ha| {
        if let Some(ha) = ha {
            ha.client.resume();
            sync_subscription(ha);
        }
    });
    request_frame();
}

#[unsafe(no_mangle)]
pub extern "C" fn on_touch() {
    request_frame();
}

fn message(msg: &str) -> Node {
    col(
        props!(background: BLACK),
        [center(
            props!(flex: 1.0),
            [text(
                msg.to_string(),
                style!(size: MESSAGE_SIZE, weight: FontWeight::REGULAR, color: GRAY_30, line_height: 1.0),
            )],
        )],
    )
}

fn tile(index: usize, name: String, state: String, active: bool) -> Node {
    let key = format!("tile_{index}");
    let weight = if active { FontWeight::BOLD } else { FontWeight::REGULAR };
    col(
        props!(flex: 1.0, background: GRAY_100, padding: 12.0, gap: 4.0, justify_content: Justify::Center),
        vec![
            text(
                name,
                style!(size: NAME_SIZE, weight: FontWeight::REGULAR, color: GRAY_30, line_height: 1.0, text_overflow: TextOverflow::Ellipsis),
            ),
            text(
                state,
                style!(size: STATE_SIZE, weight: weight, color: WHITE, line_height: 1.0, text_overflow: TextOverflow::Ellipsis),
            ),
            touchable(
                &key,
                props!(inset_top: 0.0, inset_right: 0.0, inset_bottom: 0.0, inset_left: 0.0),
                Vec::<Draw>::new(),
            ),
        ],
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn render(_delta_ms: u32) {
    let size = widget_size();
    let ids = wanted_ids();

    let Some((status, snapshot)) = HA.with_borrow(|ha| {
        let ha = ha.as_ref()?;
        let entities: Vec<_> = ids
            .iter()
            .map(|id| ha.sub.as_ref().and_then(|s| s.entity(id)))
            .collect();
        Some((ha.client.status(), entities))
    }) else {
        let _ = render_ui(size.width, size.height, message("Home Assistant unavailable"));
        return;
    };

    if ids.is_empty() {
        let _ = render_ui(size.width, size.height, message("Add entities in the settings"));
        return;
    }

    let mut tiles = Vec::new();
    for (i, (id, ent)) in ids.iter().zip(snapshot.iter()).enumerate() {
        let (name, state, active) = match ent {
            Some(e) if e.presence != Presence::Missing && e.state.is_some() => {
                let raw = e.state.as_deref().unwrap_or("");
                let name = e.text("friendly_name").unwrap_or(id).to_string();
                (name, state_line(raw, e.text("unit_of_measurement")), is_active(raw))
            }
            Some(e) if e.presence == Presence::Missing => (id.clone(), "Not found".to_string(), false),
            _ => (id.clone(), "…".to_string(), false),
        };
        let params = Params::current();
        let name = params
            .entities
            .get(i)
            .and_then(|p| p.label.as_deref())
            .filter(|l| !l.trim().is_empty())
            .map_or(name, str::to_string);
        tiles.push(tile(i, name, state, active));
    }

    let rows: Vec<Node> = tiles
        .chunks(COLUMNS)
        .map(<[Node]>::to_vec)
        .map(|chunk| row(props!(flex: 1.0, gap: GAP), chunk))
        .collect();
    let mut children = rows;
    if !matches!(status, Status::Ready) {
        children.push(text(
            format!("{status:?}"),
            style!(size: 14, weight: FontWeight::REGULAR, color: GRAY_30, line_height: 1.0),
        ));
    }
    let result = render_ui(
        size.width,
        size.height,
        col(props!(background: BLACK, padding: GAP, gap: GAP), children),
    );

    for (i, id) in ids.iter().enumerate() {
        if !result.clicks.contains_key(format!("tile_{i}").as_str()) {
            continue;
        }
        let Some(tap) = tap_for(id) else { continue };
        HA.with_borrow(|ha| {
            if let Some(ha) = ha {
                if ha.client.call_service(tap.domain, tap.service, id, &ServiceData::new()).is_err() {
                    log_error!("home-control: service call refused");
                }
            }
        });
    }
}
