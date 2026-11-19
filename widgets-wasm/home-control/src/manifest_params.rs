// AUTO-GENERATED FROM ../manifest.json by `bmc-widget-codegen` v0.1.0.
// Do not edit by hand. Run `just wasm::gen <widget>` after changing the manifest.

#![allow(
    dead_code,
    reason = "fields are widget-specific; not every key is used by every render path"
)]

use bmc_wasm_sdk::params as snapshot;
use bmc_wasm_sdk::params::typed::ParamRead;
#[derive(Clone, Debug, PartialEq)]
pub struct EntitiesItem {
    pub entity_id: String,
    pub label: Option<String>,
}
impl snapshot::typed::ValueRead for EntitiesItem {
    fn from_value(value: snapshot::Value<'_>) -> Result<Self, snapshot::typed::ReadError> {
        let row = value
            .as_object()
            .ok_or(snapshot::typed::ReadError::Mismatch)?;
        Ok(Self {
            entity_id: snapshot::typed::required_field(&row, "entity_id")?,
            label: snapshot::typed::optional_field(&row, "label")?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub entities: Vec<EntitiesItem>,
}
impl Params {
    /// Materialise a typed snapshot from a dynamic [`snapshot::Params`].
    #[must_use]
    pub fn from_snapshot(snap: &snapshot::Params) -> Self {
        Self {
            entities: <Vec<EntitiesItem> as ParamRead>::read_required(snap, "entities"),
        }
    }
    /// Latest typed snapshot delivered for this widget instance.
    /// Cached per-thread; only re-parses when `snapshot::version()` changes
    /// since the last call.
    #[must_use]
    pub fn current() -> Self {
        thread_local! {
            static CACHE : core::cell::RefCell < Option < (u64, Params) >> = const {
            core::cell::RefCell::new(None) };
        }
        let v = snapshot::version();
        CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            if let Some((cv, ref params)) = *cache
                && cv == v
            {
                return params.clone();
            }
            let fresh = Self::from_snapshot(&snapshot::current());
            *cache = Some((v, fresh.clone()));
            fresh
        })
    }
    /// Snapshot delivered immediately before [`current`]; `None` until at
    /// least one update has been observed (i.e. during `init` and the
    /// first `render`).
    #[must_use]
    pub fn previous() -> Option<Self> {
        let prev = snapshot::previous();
        if prev.is_empty() {
            None
        } else {
            Some(Self::from_snapshot(&prev))
        }
    }
    /// Manifest keys whose value differs between `self` and `other`.
    ///
    /// Intended for `on_params_update` diffing — pass `current()` and the
    /// inside-hook value of `previous()` to get the set of keys to react
    /// to. Field-by-field `PartialEq`; emitted in struct-field order so
    /// the result is deterministic.
    #[must_use]
    pub fn changed_keys(&self, other: &Self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.entities != other.entities {
            out.push("entities");
        }
        out
    }
}
/// Credential slots this widget declares, one module per slot.
pub mod credentials {
    ///Home Assistant — a `home-assistant` account. Required — the widget cannot work until an account is bound.
    ///
    ///Instance address and long-lived access token
    pub mod ha {
        ///Placeholder for this slot's `token` field.
        pub const TOKEN: &str = "{{ credential.ha.token }}";
        ///Placeholder for this slot's `url` field.
        pub const URL: &str = "{{ credential.ha.url }}";
        ///Placeholder for this slot's `websocket_url` field.
        pub const WEBSOCKET_URL: &str = "{{ credential.ha.websocket_url }}";
    }
}
