#!/usr/bin/env python3
"""Generate widgets-wasm/home-control/manifest.json with a dropdown of your Home Assistant entities.

The Deck's settings UI only knows fixed option lists, so the list is baked into the manifest:
run this again (and rebuild the .nar) whenever you add entities in Home Assistant.

  python3 tools/gen_manifest.py --entities local/entities.json     # from a saved list
  HA_TOKEN=... python3 tools/gen_manifest.py --ha-url http://192.168.1.10:8123   # straight from HA

The token is only read from the environment and never written anywhere.
"""
import argparse, json, os, re, sys, urllib.request

SLOTS = 24
LIMIT = 262144  # bytes, enforced by the Deck firmware
DOMAINS = ["light", "switch", "cover", "lock", "fan", "script", "scene", "climate", "input_boolean"]
BINARY = ["window", "door", "opening", "garage_door", "moisture", "smoke", "motion", "vibration", "occupancy", "presence", "tamper"]
SENSOR = ["temperature", "humidity", "carbon_dioxide"]
# Entities nobody wants on a wall display; the settings UI stays usable.
NOISE = re.compile(r"docker|repeater|hashboard|teplota zařízení|_chip_|prevraceno|detsky_zamek|sitova_led|rezim_turbo|odpojit_rele", re.I)
GROUPS = [("Okna a dveře", lambda d, c: d == "binary_sensor" and c in ("window", "door", "opening", "garage_door")),
          ("Senzory", lambda d, c: d == "binary_sensor"),
          ("Světla", lambda d, c: d == "light"),
          ("Spínače a zásuvky", lambda d, c: d in ("switch", "input_boolean", "fan")),
          ("Rolety, vrata, zámky", lambda d, c: d in ("cover", "lock")),
          ("Teplota a vlhkost", lambda d, c: d in ("sensor", "climate")),
          ("Scény a skripty", lambda d, c: True)]


LED_COLORS = [("Red", "FF0000"), ("Orange", "FF8000"), ("Yellow", "FFD000"), ("Green", "00D000"),
              ("Cyan", "00C8FF"), ("Blue", "0040FF"), ("Purple", "A000FF"), ("White", "FFFFFF")]
LED_EFFECTS = [("Breathing", "breathe"), ("Solid", "solid"), ("Chase", "chase"), ("Scan", "scan")]
LED_PARAMS = {
    "led_notify": {"name": "LED notification on state change", "type": "boolean", "default_value": False,
                   "description": "Light the Deck's LED strip for a few seconds when a switch, light, window, door, lock or cover changes state. "
                                  "Temperatures and other numbers never trigger it."},
    "led_color": {"name": "LED colour", "type": "string", "default_value": "FF0000",
                  "enum_values": [{"value": v, "label": n} for n, v in LED_COLORS]},
    "led_effect": {"name": "LED effect", "type": "string", "default_value": "breathe",
                   "enum_values": [{"value": v, "label": n} for n, v in LED_EFFECTS]},
    "led_seconds": {"name": "LED duration (seconds)", "type": "integer", "default_value": 5, "min": 1, "max": 60},
    "led_alerts_only": {"name": "LED only for open and alarm states", "type": "boolean", "default_value": False,
                        "description": "Skip lights and switches: notify only when a window, door, lock or cover opens, or smoke, a leak or tampering is detected."},
}


def from_ha(url, token):
    req = urllib.request.Request(url.rstrip("/") + "/api/states", headers={"Authorization": "Bearer " + token})
    with urllib.request.urlopen(req, timeout=20) as r:
        states = json.load(r)
    out = []
    for s in states:
        eid, a = s["entity_id"], s.get("attributes", {})
        d, c = eid.split(".")[0], a.get("device_class") or ""
        if d in DOMAINS or (d == "binary_sensor" and c in BINARY) or (d == "sensor" and c in SENSOR):
            out.append({"entity_id": eid, "name": a.get("friendly_name") or eid, "device_class": c})
    return out


def group_of(e):
    d, c = e["entity_id"].split(".")[0], e.get("device_class", "")
    return next(i for i, (_, test) in enumerate(GROUPS) if test(d, c))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--entities", help="JSON list of {entity_id, name, device_class}")
    ap.add_argument("--ha-url")
    ap.add_argument("--manifest", default="widgets-wasm/home-control/manifest.json")
    a = ap.parse_args()
    if a.entities:
        ents = json.load(open(a.entities))
    elif a.ha_url and os.environ.get("HA_TOKEN"):
        ents = from_ha(a.ha_url, os.environ["HA_TOKEN"])
    else:
        sys.exit("give --entities FILE, or --ha-url with HA_TOKEN set")
    ents = [e for e in ents if not NOISE.search(e["entity_id"] + " " + e["name"])]
    ents.sort(key=lambda e: (group_of(e), e["name"].lower()))
    names = [e["name"] for e in ents]
    # The firmware caps manifest.json at 256 KiB and the option list is repeated per slot, so labels stay short;
    # only a name shared by several entities gets its id appended.
    options = [{"value": e["entity_id"], "label": e["name"] if names.count(e["name"]) == 1 else f'{e["name"]} ({e["entity_id"]})'}
               for e in ents]
    m = json.load(open(a.manifest))
    params = {"ha_url": m["params"]["ha_url"]}
    for i in range(1, SLOTS + 1):
        params[f"e{i:02}"] = {"name": f"Entity {i}", "type": "string", "optional": True,
                              "description": "Shown as a tile, in this order. Leave empty to hide." if i == 1 else None,
                              "enum_values": options}
        if params[f"e{i:02}"]["description"] is None:
            del params[f"e{i:02}"]["description"]
    params.update(LED_PARAMS)
    m["params"] = params
    text = json.dumps(m, ensure_ascii=False, separators=(",", ":")) + "\n"
    size = len(text.encode())
    if size > LIMIT:
        sys.exit(f"manifest would be {size} bytes, over the firmware's {LIMIT}-byte limit: pick fewer entities (trim local/entities.json) or lower SLOTS")
    open(a.manifest, "w").write(text)
    print(f"{len(options)} entities x {SLOTS} slots -> {a.manifest} ({size} of {LIMIT} bytes)")


main()
