# deckssistant

Home Assistant widget for the [Braiins Deck](https://braiinsforge.com/hardware/braiins-deck) (firmware 26.10+).
Shows tiles for chosen entities; a tap toggles lights, switches, fans, covers and automations, runs scenes and
scripts, and presses buttons.

Built on the SDK and Home Assistant client from [BraiinsForge/bmc-main](https://github.com/BraiinsForge/bmc-main).
Licensed GPL-3.0-or-later, like that repository.

## Install on a Deck

1. Download [`home-control.nar`](home-control.nar).
2. On the Deck: **Display Widgets → Install Widget**, accept the third-party notice, pick the file.
3. In the device web UI, add a **Home Assistant** account (`http://` address + Long-Lived Access Token) and bind it
   to the widget's `Home Assistant` slot.
4. Add the entities (e.g. `light.living_room`) in the widget settings.

Home Assistant 2024.3 or newer is required. `https://` addresses are not supported by the Deck yet.

## Build

The widget lives in the `bmc-main` workspace. Clone `bmc-main`, copy `widgets-wasm/home-control` into its
`widgets-wasm/`, add `"home-control"` to the workspace `members` in `widgets-wasm/Cargo.toml`, then:

```sh
cd widgets-wasm
cargo build -p home-control --release --target wasm32-unknown-unknown
```

If you change `manifest.json`, regenerate `src/manifest_params.rs` with `just wasm::gen home-control`.

## Package (.nar without Nix)

Lay out `lib/bmc-widgets/<maintainer>/home-control/{manifest.json,bin/home-control,lib/wasm/home-control.wasm}` as in
`docs/uploading-wasm-widgets.md` of `bmc-main`, then:

```sh
python3 tools/mknar.py path/to/package-root home-control.nar
```
