//! The widget's settings. Written by hand, not generated: the entity slots are plain strings,
//! so the entity list can change in `manifest.json` (see `tools/gen_manifest.py`) without recompiling.

use bmc_wasm_sdk::params as snapshot;
use bmc_wasm_sdk::params::typed::ParamRead;

/// Entity dropdowns `e01` … `e24` in the manifest.
pub const SLOTS: usize = 24;

#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub ha_url: String,
    /// The entity ids chosen in the slots, in slot order, empty slots skipped.
    pub entities: Vec<String>,
}

impl Params {
    #[must_use]
    pub fn from_snapshot(snap: &snapshot::Params) -> Self {
        Self {
            ha_url: <String as ParamRead>::read_required(snap, "ha_url"),
            entities: (1..=SLOTS)
                .filter_map(|i| <String as ParamRead>::read_optional(snap, &format!("e{i:02}")))
                .collect(),
        }
    }

    /// Latest snapshot, parsed again only when the host delivers a new one.
    #[must_use]
    pub fn current() -> Self {
        thread_local! {
            static CACHE: core::cell::RefCell<Option<(u64, Params)>> = const { core::cell::RefCell::new(None) };
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
}
