# deckssistant

Home Assistant widget for the [Braiins Deck](https://braiinsforge.com/hardware/braiins-deck) (firmware 26.10.1+).
Shows up to 24 tiles with the entity's Home Assistant icon (Material Design Icons) and a readable state: windows and
doors say "Otevřeno" / "Zavřeno", locks "Zamčeno" / "Odemčeno", sensors show value and unit. Active tiles are coloured by kind: lights yellow, motion sensors and windows/doors blue, garage doors and covers
purple; other things that are on show green, an unlocked lock amber, and smoke, leak and tamper alarms red. A tap toggles lights, switches, fans, covers and automations, runs
scenes and scripts, and presses buttons; sensors, windows and doors are read-only.

Built on the SDK and Home Assistant client from [BraiinsForge/bmc-main](https://github.com/BraiinsForge/bmc-main).
Licensed GPL-3.0-or-later, like that repository.

## Install on a Deck

1. Pick the entities you want to offer (below), or use the ready `home-control.nar` with a few example entities.
2. On the Deck: **Display Widgets → Install Widget**, accept the third-party notice, pick the `.nar`.
3. In the device web UI, add a **generic token** account whose token is a Home Assistant Long-Lived Access Token
   (Profile -> Security in Home Assistant) and bind it to the widget's `Home Assistant token` slot.
4. In the widget settings, set **Home Assistant address** (e.g. `http://192.168.1.10:8123`) and choose entities in the
   dropdowns `Entity 1` … `Entity 24` (tile order = slot order).

The widget polls the Home Assistant REST API (`GET /api/states/<entity>` per tile every 6 s) and taps call
`/api/services/<domain>/<service>`. Use an IP address or a name the Deck can resolve. `https://` is not supported.

Firmware 26.10.1 does not know the `home-assistant` credential type, so this widget uses `generic-token`.

## LED notification

The Deck's LED strip can announce a change. In the widget settings turn on **LED notification on state change** and
pick the colour, effect and duration (1–60 s). It fires when a light, switch, window/door sensor, lock or cover
changes state while the widget polls, whether or not this widget is the scene on screen (it uses the LED's global
tier, which any scene's own LED effect outranks). **LED only for open and alarm states** narrows it to windows, doors,
locks and covers opening and to smoke, leak and tamper sensors. Temperatures and other numbers never fire it, a change
to or from "unavailable" is ignored, and neither is the first answer after the widget starts. A tap on the Deck itself
fires it once Home Assistant confirms the new state. Changing any LED setting lights the strip once as a preview;
watch `/var/log/bmc/*.log` for `home-control: LED …` lines if nothing lights.

## Your own entity list

The Deck's settings UI only knows fixed option lists, so the dropdowns are baked into `manifest.json`. The wasm does
not depend on the list: regenerate the manifest and repack, no compiler needed.

```sh
# list straight from Home Assistant (the token is read from the environment, never stored) ...
HA_TOKEN=... python3 tools/gen_manifest.py --ha-url http://192.168.1.10:8123
# ... or from a saved list like tools/entities.example.json
python3 tools/gen_manifest.py --entities my-entities.json

python3 tools/make_nar.py --manifest widgets-wasm/home-control/manifest.json --out my-home-control.nar
```

Included domains: lights, switches, fans, covers, locks, climate, scripts, scenes, and binary sensors of the classes
window, door, opening, garage_door, moisture, smoke, motion, vibration, occupancy and tamper, plus temperature, humidity
and CO2 sensors. The firmware limits `manifest.json` to 256 KiB (about 80 entities at 24 slots); the script stops
if the list is too long. Keep a personalised `.nar` out of git: it carries your entity names.

## Build

The widget lives in the `bmc-main` workspace. Clone `bmc-main`, copy `widgets-wasm/home-control` into its
`widgets-wasm/`, add `"home-control"` to the workspace `members` in `widgets-wasm/Cargo.toml`, then:

```sh
cd widgets-wasm
cargo build -p home-control --release --target wasm32-unknown-unknown
# the firmware rejects a wasm that still carries its embedded icons, so strip them out into asset files:
cargo run -p bmc-wasm-assets -- extract --input target/wasm32-unknown-unknown/release/home_control.wasm \
  --wasm-output <this repo>/prebuilt/home-control.wasm --asset-root <this repo>/prebuilt/assets \
  --artifact-root target/wasm32-unknown-unknown/release/deps
```

`prebuilt/home-control.wasm` and `prebuilt/assets/` are what `tools/make_nar.py` packs. `tools/nar.py` is a small Nix-archive reader/writer
(`ls`, `unpack`, `pack`) for when Nix is not installed.

## Icons

`widgets-wasm/home-control/assets/` holds a selection of [Material Design Icons](https://pictogrammers.com/library/mdi/)
(Apache-2.0), the set Home Assistant itself uses. An entity's own `mdi:` icon is shown if it is in the selection;
otherwise the icon follows its type (window, door, light, temperature, …). Icons from other sets (`ios:…`) fall back
to the type icon.
