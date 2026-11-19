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
//! with one `GET /api/states/<entity>` per tile, and a tap posts a service call.
//! Entities are picked in the settings UI (one dropdown per tile); icons are the bundled MDI set.
//! No Jinja template is sent: the host reads every `{{ … }}` in a request as a credential reference.

use std::cell::RefCell;

use bmc_wasm_sdk::*;

use crate::icons;
use crate::logic::{
    Tone, base_url, columns, default_icon, display_state, mdi_name, service_request, tap_for, toggled_state, tone,
    valid_entity_id,
};
use crate::manifest_params::Params;

const POLL_MS: u32 = 6_000;
const RETRY_MS: u32 = 15_000;
/// The host allows 16 fetches in flight, queued ones included; stay well under it so a tap always fits.
const WINDOW: usize = 6;
const STAGGER_MS: u32 = 60;
const GAP: f32 = 8.0;
const MESSAGE_SIZE: u32 = 22;
const AUTH: &str = "Authorization: Bearer {{ credential.ha.token }}\nContent-Type: application/json";

const TEXT_DIM: Color = Color::from_hex(0x6B_6B_6B);
const TEXT_IDLE: Color = Color::from_hex(0xB8_B8_B8);
const GREEN: Color = Color::from_hex(0x34_C0_6A);
const AMBER: Color = Color::from_hex(0xF5_A5_24);
const RED: Color = Color::from_hex(0xFF_4D_4F);

#[derive(Clone, Default)]
struct Tile {
    name: Option<String>,
    state: Option<String>,
    unit: Option<String>,
    device_class: Option<String>,
    icon: Option<String>,
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
    inflight: Vec<(FetchRequestId, usize)>,
    /// The next entity to ask about in the cycle under way.
    next: usize,
    /// The worst failure seen in the cycle under way.
    failure: Option<Health>,
}

thread_local! {
    static STATE: RefCell<State> = const {
        RefCell::new(State { tiles: Vec::new(), health: Health::Loading, inflight: Vec::new(), next: 0, failure: None })
    };
}

fn config() -> (Option<String>, Vec<String>) {
    let p = Params::current();
    let mut ids: Vec<String> = Vec::new();
    for id in p.entities {
        if valid_entity_id(&id) && !ids.contains(&id) {
            ids.push(id);
        }
    }
    (base_url(&p.ha_url), ids)
}

fn severity(health: Health) -> u8 {
    match health {
        Health::NotConfigured => 3,
        Health::BadToken => 2,
        Health::Unreachable => 1,
        _ => 0,
    }
}

fn send_state_get(s: &mut State, base: &str, ids: &[String], i: usize, delay_ms: u32) {
    let Some(entity) = ids.get(i) else { return };
    let url = format!("{base}/api/states/{entity}");
    match FetchRequest::get(&url).headers(AUTH).send_after(delay_ms, on_state) {
        Some(id) => s.inflight.push((id, i)),
        None => s.failure = Some(Health::Unreachable),
    }
}

/// Cancel whatever is queued and ask Home Assistant about every entity, after `delay_ms`.
/// Only `WINDOW` requests are out at a time; each answer releases the next one.
fn poll(delay_ms: u32) {
    let (base, ids) = config();
    STATE.with_borrow_mut(|s| {
        for (id, _) in s.inflight.drain(..) {
            let _ = cancel(id);
        }
        s.failure = None;
        s.next = 0;
        s.tiles.resize(ids.len(), Tile::default());
        let Some(base) = base.filter(|_| !ids.is_empty()) else {
            s.health = Health::NotConfigured;
            s.tiles.clear();
            return;
        };
        let first = ids.len().min(WINDOW);
        for i in 0..first {
            let stagger = STAGGER_MS * u32::try_from(i).unwrap_or(0);
            send_state_get(s, &base, &ids, i, delay_ms.saturating_add(stagger));
        }
        s.next = first;
        if s.inflight.is_empty() {
            s.health = Health::Unreachable;
        }
    });
    request_frame();
}

fn on_state(response: &FetchResponse) {
    let mut next = None;
    STATE.with_borrow_mut(|s| {
        let Some(pos) = s.inflight.iter().position(|(id, _)| *id == response.request_id) else {
            return;
        };
        let (_, index) = s.inflight.remove(pos);
        let result = match response.outcome() {
            Some(FetchOutcome::Http(200..=299)) => {
                let doc = JsonDoc::parse(response.body());
                if let (true, Some(tile)) = (doc.is_valid(), s.tiles.get_mut(index)) {
                    *tile = Tile {
                        state: doc.str("/state"),
                        name: doc.str("/attributes/friendly_name"),
                        unit: doc.str("/attributes/unit_of_measurement"),
                        device_class: doc.str("/attributes/device_class"),
                        icon: doc.str("/attributes/icon"),
                    };
                    Health::Ok
                } else {
                    Health::Unreachable
                }
            }
            Some(FetchOutcome::Http(404)) => {
                if let Some(tile) = s.tiles.get_mut(index) {
                    *tile = Tile { state: Some("unavailable".to_string()), ..Tile::default() };
                }
                Health::Ok
            }
            Some(FetchOutcome::Http(401 | 403)) => Health::BadToken,
            Some(FetchOutcome::Refused) => Health::NotConfigured,
            _ => Health::Unreachable,
        };
        if severity(result) > s.failure.map_or(0, severity) {
            s.failure = Some(result);
        }
        let (base, ids) = config();
        if let Some(base) = base
            && s.next < ids.len()
        {
            let i = s.next;
            s.next += 1;
            send_state_get(s, &base, &ids, i, 0);
        }
        if s.inflight.is_empty() {
            s.health = s.failure.take().unwrap_or(Health::Ok);
            next = Some(if s.health == Health::Ok { POLL_MS } else { RETRY_MS });
        }
    });
    match next {
        Some(delay) => poll(delay),
        None => request_frame_after(100),
    }
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
    // A touch also retries a cycle that could not even be queued.
    if STATE.with_borrow(|s| s.inflight.is_empty() && s.health == Health::Unreachable) {
        poll(0);
    }
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

/// Text and icon sizes that grow as the grid gets sparser.
struct Scale {
    icon: f32,
    name: u32,
    state: u32,
}

fn scale(cols: usize) -> Scale {
    match cols {
        0..=2 => Scale { icon: 64.0, name: 24, state: 36 },
        3..=4 => Scale { icon: 44.0, name: 17, state: 26 },
        5 => Scale { icon: 36.0, name: 15, state: 22 },
        _ => Scale { icon: 30.0, name: 13, state: 19 },
    }
}

fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Dim => TEXT_DIM,
        Tone::Off => TEXT_IDLE,
        Tone::On => GREEN,
        Tone::Warn => AMBER,
        Tone::Alert => RED,
    }
}

fn icon_for(id: &str, t: &Tile) -> Svg {
    let state = t.state.as_deref().unwrap_or("unknown");
    let own = t.icon.as_deref().and_then(mdi_name).and_then(icons::by_name);
    own.or_else(|| icons::by_name(default_icon(id, t.device_class.as_deref(), state)))
        .or_else(|| icons::by_name("help-circle-outline"))
        .expect("BUG: help-circle-outline is bundled")
}

fn tile(index: usize, id: &str, t: &Tile, sc: &Scale) -> Node {
    let key = format!("tile_{index}");
    let raw = t.state.as_deref();
    let tone = raw.map_or(Tone::Dim, |s| tone(id, t.device_class.as_deref(), s));
    let color = tone_color(tone);
    let name = t.name.clone().unwrap_or_else(|| id.to_string());
    let line = raw.map_or_else(
        || "…".to_string(),
        |s| display_state(id, t.device_class.as_deref(), s, t.unit.as_deref()),
    );
    let weight = if matches!(tone, Tone::On | Tone::Warn | Tone::Alert) { FontWeight::BOLD } else { FontWeight::REGULAR };
    let icon = icon_for(id, t);
    row(
        props!(flex: 1.0, background: GRAY_100, padding: 10.0, gap: 10.0, border_radius: 8.0, cross_align: CrossAlign::Center),
        vec![
            canvas(
                props!(width: sc.icon, height: sc.icon),
                [Draw::svg_contain(&icon, sc.icon, color).with_anti_alias()],
            ),
            col(
                props!(flex: 1.0, gap: 4.0, justify_content: Justify::Center),
                vec![
                    text(
                        name,
                        style!(size: sc.name, weight: FontWeight::REGULAR, color: TEXT_IDLE, line_height: 1.0, text_overflow: TextOverflow::Ellipsis),
                    ),
                    text(
                        line,
                        style!(size: sc.state, weight: weight, color: color, line_height: 1.0, text_overflow: TextOverflow::Ellipsis),
                    ),
                ],
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
    let (base, ids) = config();
    let (tiles, health) = STATE.with_borrow(|s| (s.tiles.clone(), s.health));

    let note = match health {
        _ if base.is_none() => Some("Nastav adresu Home Assistantu (http://IP:8123) v nastavení widgetu"),
        _ if ids.is_empty() => Some("Vyber entity v nastavení widgetu"),
        Health::NotConfigured => Some("Přiřaď widgetu token Home Assistantu"),
        Health::BadToken => Some("Home Assistant odmítl token"),
        Health::Unreachable if tiles.is_empty() => Some("Home Assistant není dostupný"),
        Health::Loading => Some("Připojuji…"),
        _ => None,
    };
    if let (Some(note), true) = (note, tiles.is_empty()) {
        let _ = render_ui(size.width, size.height, message(note));
        return;
    }

    let cols = columns(ids.len());
    let sc = scale(cols);
    let mut nodes: Vec<Node> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| tile(i, id, &tiles.get(i).cloned().unwrap_or_default(), &sc))
        .collect();
    while nodes.len() % cols != 0 {
        nodes.push(col(props!(flex: 1.0), Vec::<Node>::new()));
    }
    let mut children: Vec<Node> = nodes
        .chunks(cols)
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
            continue;
        }
        // Show the new state at once; the poll that follows the call confirms or corrects it.
        STATE.with_borrow_mut(|s| {
            if let Some(t) = s.tiles.get_mut(i)
                && let Some(flipped) = t.state.as_deref().and_then(toggled_state)
            {
                t.state = Some(flipped.to_string());
            }
        });
        request_frame();
    }
}
