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

//! The entry points the host calls. The widget polls Home Assistant's REST API
//! with one `/api/template` request for all tiles, and a tap posts a service call.

use std::cell::RefCell;

use bmc_wasm_sdk::*;

use crate::logic::{
    base_url, is_active, service_request, state_line, tap_for, template_request, valid_entity_id,
};
use crate::manifest_params::Params;

const POLL_MS: u32 = 5_000;
const RETRY_MS: u32 = 15_000;
const COLUMNS: usize = 4;
const GAP: f32 = 8.0;
const NAME_SIZE: u32 = 16;
const STATE_SIZE: u32 = 22;
const MESSAGE_SIZE: u32 = 22;
const AUTH: &str = "Authorization: Bearer {{ credential.ha.token }}\nContent-Type: application/json";

#[derive(Clone, Default)]
struct Tile {
    name: Option<String>,
    state: Option<String>,
    unit: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Health {
    Loading,
    Ok,
    BadToken,
    Unreachable,
    NotConfigured,
}

struct State {
    tiles: Vec<Tile>,
    health: Health,
    pending: Option<FetchRequestId>,
}

thread_local! {
    static STATE: RefCell<State> = const {
        RefCell::new(State { tiles: Vec::new(), health: Health::Loading, pending: None })
    };
}

fn config() -> (Option<String>, Vec<String>) {
    let params = Params::current();
    let ids = params
        .entities
        .iter()
        .map(|e| e.entity_id.trim().to_string())
        .filter(|id| valid_entity_id(id))
        .collect();
    (base_url(&params.ha_url), ids)
}

/// Cancel whatever is queued and ask Home Assistant now, or after `delay_ms`.
fn poll(delay_ms: u32) {
    let (base, ids) = config();
    STATE.with_borrow_mut(|s| {
        if let Some(id) = s.pending.take() {
            let _ = cancel(id);
        }
        let Some(base) = base.filter(|_| !ids.is_empty()) else {
            s.health = Health::NotConfigured;
            s.tiles.clear();
            return;
        };
        let url = format!("{base}/api/template");
        let body = template_request(&ids);
        s.pending = FetchRequest::post(&url)
            .headers(AUTH)
            .body(body.as_bytes())
            .send_after(delay_ms, on_states);
    });
    request_frame();
}

fn on_states(response: &FetchResponse) {
    let (_, ids) = config();
    let mut next = RETRY_MS;
    STATE.with_borrow_mut(|s| {
        s.pending = None;
        match response.outcome() {
            Some(FetchOutcome::Http(200..=299)) => {
                let doc = JsonDoc::parse(response.body());
                let count = doc.len("").unwrap_or(0);
                if doc.is_valid() && count == ids.len() {
                    s.tiles = (0..count)
                        .map(|i| Tile {
                            state: doc.str(&format!("/{i}/s")),
                            name: doc.str(&format!("/{i}/n")),
                            unit: doc.str(&format!("/{i}/u")),
                        })
                        .collect();
                    s.health = Health::Ok;
                    next = POLL_MS;
                } else {
                    s.health = Health::Unreachable;
                }
            }
            Some(FetchOutcome::Http(401 | 403)) => s.health = Health::BadToken,
            Some(FetchOutcome::Refused) => s.health = Health::NotConfigured,
            _ => s.health = Health::Unreachable,
        }
    });
    poll(next);
}

fn on_service_done(_response: &FetchResponse) {
    poll(300);
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    poll(0);
}

#[unsafe(no_mangle)]
pub extern "C" fn on_params_update() {
    STATE.with_borrow_mut(|s| s.tiles.clear());
    poll(0);
}

#[unsafe(no_mangle)]
pub extern "C" fn on_credentials_update() {
    poll(0);
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
    let params = Params::current();
    let (base, ids) = config();
    let (tiles, health) = STATE.with_borrow(|s| (s.tiles.clone(), s.health));

    let note = match health {
        _ if base.is_none() => Some("Set the Home Assistant address (http://IP:8123) in the settings"),
        _ if ids.is_empty() => Some("Add entities in the settings"),
        Health::NotConfigured => Some("Bind a Home Assistant token to this widget"),
        Health::BadToken => Some("Home Assistant rejected the token"),
        Health::Unreachable if tiles.is_empty() => Some("Cannot reach Home Assistant"),
        Health::Loading => Some("Connecting…"),
        _ => None,
    };
    if let (Some(note), true) = (note, tiles.is_empty()) {
        let _ = render_ui(size.width, size.height, message(note));
        return;
    }

    let mut nodes = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let t = tiles.get(i).cloned().unwrap_or_default();
        let label = params
            .entities
            .iter()
            .filter(|e| valid_entity_id(e.entity_id.trim()))
            .nth(i)
            .and_then(|e| e.label.as_deref())
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string);
        let name = label.or(t.name).unwrap_or_else(|| id.clone());
        let raw = t.state.as_deref();
        let line = raw.map_or_else(|| "…".to_string(), |s| state_line(s, t.unit.as_deref()));
        nodes.push(tile(i, name, line, raw.is_some_and(is_active)));
    }
    let mut children: Vec<Node> = nodes
        .chunks(COLUMNS)
        .map(<[Node]>::to_vec)
        .map(|chunk| row(props!(flex: 1.0, gap: GAP), chunk))
        .collect();
    if let Some(note) = note {
        children.push(text(
            note.to_string(),
            style!(size: 14, weight: FontWeight::REGULAR, color: GRAY_30, line_height: 1.0),
        ));
    }
    let result = render_ui(
        size.width,
        size.height,
        col(props!(background: BLACK, padding: GAP, gap: GAP), children),
    );

    let Some(base) = base else { return };
    for (i, id) in ids.iter().enumerate() {
        if !result.clicks.contains_key(format!("tile_{i}").as_str()) {
            continue;
        }
        let Some(tap) = tap_for(id) else { continue };
        let url = format!("{base}/api/services/{}/{}", tap.domain, tap.service);
        let body = service_request(id);
        let sent = FetchRequest::post(&url)
            .headers(AUTH)
            .body(body.as_bytes())
            .send(on_service_done);
        if sent.is_none() {
            log_error!("home-control: service call not queued");
        }
    }
}
