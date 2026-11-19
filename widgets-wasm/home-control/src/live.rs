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
    Accent, Tone, accent, base_url, columns, default_icon, display_state, mdi_name, service_request, led_trigger, parse_hex_color, tap_for, toggled_state, tone,
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
const YELLOW: Color = Color::from_hex(0xFF_C8_3D);
const BLUE: Color = Color::from_hex(0x3D_9B_FF);
const PURPLE: Color = Color::from_hex(0xB3_6B_FF);
const PRESSED_BG: Color = Color::from_hex(0x3A_3A_3A);
/// How long a tile stays lit after the finger lifts, so a quick tap is still seen.
const FLASH_MS: u32 = 220;
/// After a tap, ask Home Assistant about that entity alone: first soon, then every half second.
const CONFIRM_FIRST_MS: u32 = 350;
const CONFIRM_NEXT_MS: u32 = 500;
/// Give up waiting for the state to change after this many answers and show what Home Assistant says.
const CONFIRM_MAX: u32 = 6;

#[derive(Clone, Default)]
struct Tile {
    name: Option<String>,
    state: Option<String>,
    unit: Option<String>,
    device_class: Option<String>,
    icon: Option<String>,
}

/// A tap whose result is still being confirmed. While it lasts the tapped tile keeps its optimistic
/// state, so an answer from before Home Assistant caught up cannot flip it back.
struct Pending {
    index: usize,
    entity: String,
    expect: Option<&'static str>,
    before: Option<String>,
    attempts: u32,
    post_tries: u32,
    confirm: Option<FetchRequestId>,
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
    /// The tile under the finger right now.
    pressed: Option<usize>,
    pending: Option<Pending>,
    /// The tile just tapped and the time left on its highlight.
    flash: Option<(usize, u32)>,
    /// The worst failure seen in the cycle under way.
    failure: Option<Health>,
}

thread_local! {
    static STATE: RefCell<State> = const {
        RefCell::new(State { tiles: Vec::new(), health: Health::Loading, inflight: Vec::new(), next: 0, pressed: None, pending: None, flash: None, failure: None })
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

/// Light the strip for a few seconds in the colour chosen in the settings. The global tier shows
/// whichever scene is on screen; `stop` first so quick changes do not queue up one effect each.
fn notify_led() {
    let p = Params::current();
    if !p.led_notify {
        return;
    }
    let rgb = parse_hex_color(&p.led_color).unwrap_or(0xFF_00_00);
    let effect = match p.led_effect.as_str() {
        "solid" => led::LedEffect::Solid,
        "chase" => led::LedEffect::Chase,
        "scan" => led::LedEffect::Scan,
        _ => led::LedEffect::Breathe,
    };
    log_info!("home-control: LED {} #{} for {} s", p.led_effect, p.led_color, p.led_seconds);
    led::stop();
    led::set_effect_global(effect, Color::from_hex(rgb), 1_500, Some(p.led_seconds.clamp(1, 60) * 1_000));
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
    if STATE.with_borrow(|s| s.pending.is_some()) {
        return;
    }
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
    let mut light_led = false;
    STATE.with_borrow_mut(|s| {
        let Some(pos) = s.inflight.iter().position(|(id, _)| *id == response.request_id) else {
            return;
        };
        let (_, index) = s.inflight.remove(pos);
        let ids_now = config().1;
        let alerts_only = Params::current().led_alerts_only;
        let result = match response.outcome() {
            Some(FetchOutcome::Http(200..=299)) => {
                let doc = JsonDoc::parse(response.body());
                if let (true, Some(tile)) = (doc.is_valid(), s.tiles.get_mut(index)) {
                    let old = tile.state.take();
                    *tile = Tile {
                        state: doc.str("/state"),
                        name: doc.str("/attributes/friendly_name"),
                        unit: doc.str("/attributes/unit_of_measurement"),
                        device_class: doc.str("/attributes/device_class"),
                        icon: doc.str("/attributes/icon"),
                    };
                    // The first answer for a tile has nothing to compare with, so it never fires.
                    if let (Some(old), Some(new), Some(id)) = (old.as_deref(), tile.state.as_deref(), ids_now.get(index)) {
                        light_led = led_trigger(id, tile.device_class.as_deref(), old, new, alerts_only);
                    }
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
    if light_led {
        notify_led();
    }
    match next {
        Some(delay) => poll(delay),
        None => request_frame_after(100),
    }
}

fn send_service(base: &str, entity: &str) -> bool {
    let Some(tap) = tap_for(entity) else { return false };
    let url = format!("{base}/api/services/{}/{}", tap.domain, tap.service);
    let body = service_request(entity);
    FetchRequest::post(&url).headers(AUTH).body(body.as_bytes()).send(on_service_done).is_some()
}

/// Ask for the tapped entity alone after `delay_ms`; false when the request could not be queued.
fn schedule_confirm(s: &mut State, delay_ms: u32) -> bool {
    let (Some(base), Some(p)) = (config().0, s.pending.as_mut()) else { return false };
    if let Some(old) = p.confirm.take() {
        let _ = cancel(old);
    }
    let url = format!("{base}/api/states/{}", p.entity);
    p.confirm = FetchRequest::get(&url).headers(AUTH).send_after(delay_ms, on_confirm);
    p.confirm.is_some()
}

/// The call failed for good: put the tile back as it was.
fn revert_pending(s: &mut State) {
    let Some(p) = s.pending.take() else { return };
    log_error!("home-control: the call for {} failed", p.entity);
    if let Some(tile) = s.tiles.get_mut(p.index) {
        tile.state = p.before;
    }
}

fn on_service_done(response: &FetchResponse) {
    let ok = matches!(response.outcome(), Some(FetchOutcome::Http(200..=299)));
    let transport_failure = !matches!(response.outcome(), Some(FetchOutcome::Http(_)));
    let mut resume = false;
    STATE.with_borrow_mut(|s| {
        let Some(p) = s.pending.as_mut() else { return };
        if ok {
            resume = !schedule_confirm(s, CONFIRM_FIRST_MS);
        } else if transport_failure && p.post_tries == 0 {
            p.post_tries = 1;
            let entity = p.entity.clone();
            if !config().0.is_some_and(|base| send_service(&base, &entity)) {
                revert_pending(s);
                resume = true;
            }
        } else {
            revert_pending(s);
            resume = true;
        }
        if resume {
            s.pending = None;
        }
    });
    if resume {
        poll(300);
    }
    request_frame();
}

fn on_confirm(response: &FetchResponse) {
    let mut resume = false;
    let mut light_led = false;
    STATE.with_borrow_mut(|s| {
        let Some(p) = s.pending.as_mut() else { return };
        if p.confirm != Some(response.request_id) {
            return;
        }
        p.confirm = None;
        p.attempts += 1;
        let doc = JsonDoc::parse(response.body());
        let answered = matches!(response.outcome(), Some(FetchOutcome::Http(200..=299))) && doc.is_valid();
        let state = if answered { doc.str("/state") } else { None };
        let matched = match (p.expect, state.as_deref()) {
            (Some(want), Some(got)) => want == got,
            (None, Some(_)) => true,
            _ => false,
        };
        let done = matched || p.attempts >= CONFIRM_MAX;
        let alerts_only = Params::current().led_alerts_only;
        let index = p.index;
        if let (true, Some(tile)) = (answered, s.tiles.get_mut(index)) {
            tile.name = doc.str("/attributes/friendly_name");
            tile.unit = doc.str("/attributes/unit_of_measurement");
            tile.device_class = doc.str("/attributes/device_class");
            tile.icon = doc.str("/attributes/icon");
            if done {
                if let (true, Some(before), Some(new)) = (matched, p.before.as_deref(), state.as_deref()) {
                    light_led = led_trigger(&p.entity, tile.device_class.as_deref(), before, new, alerts_only);
                }
                tile.state = state;
            }
        }
        if done || !schedule_confirm(s, CONFIRM_NEXT_MS) {
            s.pending = None;
            resume = true;
        }
    });
    if light_led {
        notify_led();
    }
    if resume {
        poll(200);
    }
    request_frame();
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    poll(0);
}

#[unsafe(no_mangle)]
pub extern "C" fn on_params_update() {
    STATE.with_borrow_mut(|s| {
        s.tiles.clear();
        s.pending = None;
    });
    // Changing a LED setting lights the strip once, so the choice can be seen at once.
    let now = Params::current();
    if now.led_notify
        && Params::previous().is_some_and(|b| {
            !b.led_notify
                || b.led_color != now.led_color
                || b.led_effect != now.led_effect
                || b.led_seconds != now.led_seconds
        })
    {
        notify_led();
    }
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

fn tile(index: usize, id: &str, t: &Tile, sc: &Scale, lit: bool) -> Node {
    let key = format!("tile_{index}");
    let raw = t.state.as_deref();
    let tone = raw.map_or(Tone::Dim, |s| tone(id, t.device_class.as_deref(), s));
    let color = match (tone, accent(id, t.device_class.as_deref())) {
        (Tone::On | Tone::Warn, Some(Accent::Yellow)) => YELLOW,
        (Tone::On | Tone::Warn, Some(Accent::Blue)) => BLUE,
        (Tone::On | Tone::Warn, Some(Accent::Purple)) => PURPLE,
        _ => tone_color(tone),
    };
    let name = t.name.clone().unwrap_or_else(|| id.to_string());
    let line = raw.map_or_else(
        || "…".to_string(),
        |s| display_state(id, t.device_class.as_deref(), s, t.unit.as_deref()),
    );
    let weight = if matches!(tone, Tone::On | Tone::Warn | Tone::Alert) { FontWeight::BOLD } else { FontWeight::REGULAR };
    let icon = icon_for(id, t);
    row(
        props!(flex: 1.0, background: if lit { PRESSED_BG } else { GRAY_100 }, padding: 10.0, gap: 10.0, border_radius: 8.0, cross_align: CrossAlign::Center),
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
pub extern "C" fn render(delta_ms: u32) {
    let size = widget_size();
    let (base, ids) = config();
    let (tiles, health, lit) = STATE.with_borrow_mut(|s| {
        s.flash = s.flash.and_then(|(i, left)| left.checked_sub(delta_ms).filter(|l| *l > 0).map(|l| (i, l)));
        (s.tiles.clone(), s.health, (s.pressed, s.flash.map(|(i, _)| i)))
    });

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
        .map(|(i, id)| {
            let is_lit = lit.0 == Some(i) || lit.1 == Some(i);
            tile(i, id, &tiles.get(i).cloned().unwrap_or_default(), &sc, is_lit)
        })
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

    let pressed = (0..ids.len()).find(|i| result.drags.contains_key(format!("tile_{i}").as_str()));
    let tapped = (0..ids.len()).find(|i| result.clicks.contains_key(format!("tile_{i}").as_str()));
    let pressed_before = STATE.with_borrow(|s| s.pressed);
    let changed = STATE.with_borrow_mut(|s| {
        let before = (s.pressed, s.flash.map(|(i, _)| i));
        s.pressed = pressed;
        if let Some(i) = tapped {
            s.flash = Some((i, FLASH_MS));
        }
        s.flash.is_some() || before != (s.pressed, s.flash.map(|(i, _)| i))
    });
    if STATE.with_borrow(|s| s.pressed) != pressed_before {
        request_frame();
    } else if changed {
        request_frame_after(40);
    }

    let Some(base) = base else { return };
    for (i, id) in ids.iter().enumerate() {
        if !result.clicks.contains_key(format!("tile_{i}").as_str()) {
            continue;
        }
        if tap_for(id).is_none() {
            continue;
        }
        // Drop the running refresh so the call goes out first, then watch only this entity.
        let sent = STATE.with_borrow_mut(|s| {
            for (req, _) in s.inflight.drain(..) {
                let _ = cancel(req);
            }
            if let Some(old) = s.pending.take().and_then(|p| p.confirm) {
                let _ = cancel(old);
            }
            let before = s.tiles.get(i).and_then(|t| t.state.clone());
            let expect = before.as_deref().and_then(toggled_state);
            s.pending = Some(Pending { index: i, entity: id.clone(), expect, before, attempts: 0, post_tries: 0, confirm: None });
            if let (Some(want), Some(tile)) = (expect, s.tiles.get_mut(i)) {
                tile.state = Some(want.to_string());
            }
            let sent = send_service(&base, id);
            if !sent {
                revert_pending(s);
            }
            sent
        });
        if !sent {
            poll(300);
        }
        request_frame();
    }
}
