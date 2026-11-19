//! Material Design Icons (Apache-2.0, https://pictogrammers.com/library/mdi/), the set Home Assistant uses.
//! The `icon` attribute `mdi:<name>` of an entity picks one of these; others fall back to a default for the entity type.

use bmc_wasm_sdk::{Svg, include_svg};

const ACCOUNT: Svg = include_svg!("assets/account.svg");
const ALERT_CIRCLE: Svg = include_svg!("assets/alert-circle.svg");
const BATTERY: Svg = include_svg!("assets/battery.svg");
const BLINDS: Svg = include_svg!("assets/blinds.svg");
const BLINDS_OPEN: Svg = include_svg!("assets/blinds-open.svg");
const CHANDELIER: Svg = include_svg!("assets/chandelier.svg");
const DESKTOP_TOWER_MONITOR: Svg = include_svg!("assets/desktop-tower-monitor.svg");
const DOOR_CLOSED: Svg = include_svg!("assets/door-closed.svg");
const DOOR_OPEN: Svg = include_svg!("assets/door-open.svg");
const DOOR_SLIDING_LOCK: Svg = include_svg!("assets/door-sliding-lock.svg");
const FAN: Svg = include_svg!("assets/fan.svg");
const FLASH: Svg = include_svg!("assets/flash.svg");
const GARAGE_OPEN_VARIANT: Svg = include_svg!("assets/garage-open-variant.svg");
const GARAGE_VARIANT: Svg = include_svg!("assets/garage-variant.svg");
const GESTURE_TAP_BUTTON: Svg = include_svg!("assets/gesture-tap-button.svg");
const HELP_CIRCLE_OUTLINE: Svg = include_svg!("assets/help-circle-outline.svg");
const HOME: Svg = include_svg!("assets/home.svg");
const LAVA_LAMP: Svg = include_svg!("assets/lava-lamp.svg");
const LED_ON: Svg = include_svg!("assets/led-on.svg");
const LED_STRIP: Svg = include_svg!("assets/led-strip.svg");
const LED_STRIP_VARIANT: Svg = include_svg!("assets/led-strip-variant.svg");
const LIGHTBULB: Svg = include_svg!("assets/lightbulb.svg");
const LIGHTBULB_OUTLINE: Svg = include_svg!("assets/lightbulb-outline.svg");
const LOCK: Svg = include_svg!("assets/lock.svg");
const LOCK_OPEN_VARIANT: Svg = include_svg!("assets/lock-open-variant.svg");
const MOLECULE_CO2: Svg = include_svg!("assets/molecule-co2.svg");
const MOTION_SENSOR: Svg = include_svg!("assets/motion-sensor.svg");
const MOTION_SENSOR_OFF: Svg = include_svg!("assets/motion-sensor-off.svg");
const MOVIE_OPEN: Svg = include_svg!("assets/movie-open.svg");
const POWER_PLUG: Svg = include_svg!("assets/power-plug.svg");
const RADIATOR: Svg = include_svg!("assets/radiator.svg");
const ROBOT: Svg = include_svg!("assets/robot.svg");
const SCRIPT_TEXT_PLAY: Svg = include_svg!("assets/script-text-play.svg");
const SHIELD_ALERT: Svg = include_svg!("assets/shield-alert.svg");
const SMOKE_DETECTOR_VARIANT: Svg = include_svg!("assets/smoke-detector-variant.svg");
const SOLAR_PANEL: Svg = include_svg!("assets/solar-panel.svg");
const SPEAKER: Svg = include_svg!("assets/speaker.svg");
const SPOTLIGHT_BEAM: Svg = include_svg!("assets/spotlight-beam.svg");
const TELEVISION: Svg = include_svg!("assets/television.svg");
const THERMOMETER: Svg = include_svg!("assets/thermometer.svg");
const THERMOMETER_LINES: Svg = include_svg!("assets/thermometer-lines.svg");
const THERMOSTAT: Svg = include_svg!("assets/thermostat.svg");
const TOGGLE_SWITCH: Svg = include_svg!("assets/toggle-switch.svg");
const TOGGLE_SWITCH_OFF_OUTLINE: Svg = include_svg!("assets/toggle-switch-off-outline.svg");
const VIBRATE: Svg = include_svg!("assets/vibrate.svg");
const WASHING_MACHINE_ALERT: Svg = include_svg!("assets/washing-machine-alert.svg");
const WATER_ALERT: Svg = include_svg!("assets/water-alert.svg");
const WATER_PERCENT: Svg = include_svg!("assets/water-percent.svg");
const WINDOW_CLOSED_VARIANT: Svg = include_svg!("assets/window-closed-variant.svg");
const WINDOW_OPEN_VARIANT: Svg = include_svg!("assets/window-open-variant.svg");

/// The bundled icon called `name` (an MDI name without the `mdi:` prefix).
#[must_use]
pub fn by_name(name: &str) -> Option<Svg> {
    Some(match name {
        "account" => ACCOUNT,
        "alert-circle" => ALERT_CIRCLE,
        "battery" => BATTERY,
        "blinds" => BLINDS,
        "blinds-open" => BLINDS_OPEN,
        "chandelier" => CHANDELIER,
        "desktop-tower-monitor" => DESKTOP_TOWER_MONITOR,
        "door-closed" => DOOR_CLOSED,
        "door-open" => DOOR_OPEN,
        "door-sliding-lock" => DOOR_SLIDING_LOCK,
        "fan" => FAN,
        "flash" => FLASH,
        "garage-open-variant" => GARAGE_OPEN_VARIANT,
        "garage-variant" => GARAGE_VARIANT,
        "gesture-tap-button" => GESTURE_TAP_BUTTON,
        "help-circle-outline" => HELP_CIRCLE_OUTLINE,
        "home" => HOME,
        "lava-lamp" => LAVA_LAMP,
        "led-on" => LED_ON,
        "led-strip" => LED_STRIP,
        "led-strip-variant" => LED_STRIP_VARIANT,
        "lightbulb" => LIGHTBULB,
        "lightbulb-outline" => LIGHTBULB_OUTLINE,
        "lock" => LOCK,
        "lock-open-variant" => LOCK_OPEN_VARIANT,
        "molecule-co2" => MOLECULE_CO2,
        "motion-sensor" => MOTION_SENSOR,
        "motion-sensor-off" => MOTION_SENSOR_OFF,
        "movie-open" => MOVIE_OPEN,
        "power-plug" => POWER_PLUG,
        "radiator" => RADIATOR,
        "robot" => ROBOT,
        "script-text-play" => SCRIPT_TEXT_PLAY,
        "shield-alert" => SHIELD_ALERT,
        "smoke-detector-variant" => SMOKE_DETECTOR_VARIANT,
        "solar-panel" => SOLAR_PANEL,
        "speaker" => SPEAKER,
        "spotlight-beam" => SPOTLIGHT_BEAM,
        "television" => TELEVISION,
        "thermometer" => THERMOMETER,
        "thermometer-lines" => THERMOMETER_LINES,
        "thermostat" => THERMOSTAT,
        "toggle-switch" => TOGGLE_SWITCH,
        "toggle-switch-off-outline" => TOGGLE_SWITCH_OFF_OUTLINE,
        "vibrate" => VIBRATE,
        "washing-machine-alert" => WASHING_MACHINE_ALERT,
        "water-alert" => WATER_ALERT,
        "water-percent" => WATER_PERCENT,
        "window-closed-variant" => WINDOW_CLOSED_VARIANT,
        "window-open-variant" => WINDOW_OPEN_VARIANT,
        _ => return None,
    })
}
