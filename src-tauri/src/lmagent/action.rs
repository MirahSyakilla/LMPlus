//! LMPlusAction — the action abstraction layer.
//!
//! Every LMPlus→game integration goes through this enum so new actions are a
//! one-line addition here plus one handler in the agent.

use serde::{Deserialize, Serialize};

pub const FORMATION_NAMES: [&str; 6] = [
    "Infantry Phalanx",
    "Ranged Phalanx",
    "Cavalry Phalanx",
    "Infantry Wedge",
    "Ranged Wedge",
    "Cavalry Wedge",
];

pub const MAP3DVIEW_MODE_NAMES: [&str; 3] = ["Full", "Balanced", "None"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum LMPlusAction {
    /// Select one of the six troop formations (0..5).
    Formation { index: u8 },
    /// Kingdom Map 3D View: 0=Full, 1=Balanced, 2=None.
    Map3DView { mode: u8 },
    /// Arbitrary map zoom (managed camera distance).
    MapZoom { value: f32 },
    /// In-process account switch (no process restart).
    SwitchAccount,
}

impl LMPlusAction {
    pub fn to_wire(&self) -> String {
        match self {
            LMPlusAction::Formation { index } => {
                format!("{{\"action\":\"formation\",\"index\":{}}}", index)
            }
            LMPlusAction::Map3DView { mode } => {
                format!("{{\"action\":\"map3dview\",\"mode\":{}}}", mode)
            }
            LMPlusAction::MapZoom { value } => {
                format!("{{\"action\":\"zoom\",\"level\":{}}}", value)
            }
            LMPlusAction::SwitchAccount => "{\"action\":\"switch_account\"}".to_string(),
        }
    }

    pub fn human_name(&self) -> String {
        match self {
            LMPlusAction::Formation { index } => FORMATION_NAMES
                .get(*index as usize)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Formation {}", index)),
            LMPlusAction::Map3DView { mode } => MAP3DVIEW_MODE_NAMES
                .get(*mode as usize)
                .map(|s| format!("Kingdom Map 3D View: {}", s))
                .unwrap_or_else(|| format!("Map3DView {}", mode)),
            LMPlusAction::MapZoom { value } => format!("Map Zoom {:.0}%", value * 100.0),
            LMPlusAction::SwitchAccount => "Switch Account".to_string(),
        }
    }
}

/// Named-id mapping used by the Formation hotkey tab (matches the frontend's
/// DIRECT_ACTION_SPECS).
pub const NAMED_ACTIONS: [(&str, LMPlusAction); 10] = [
    ("formation_inf_phalanx", LMPlusAction::Formation { index: 0 }),
    ("formation_range_phalanx", LMPlusAction::Formation { index: 1 }),
    ("formation_cav_phalanx", LMPlusAction::Formation { index: 2 }),
    ("formation_inf_wedge", LMPlusAction::Formation { index: 3 }),
    ("formation_range_wedge", LMPlusAction::Formation { index: 4 }),
    ("formation_cav_wedge", LMPlusAction::Formation { index: 5 }),
    ("map3dview_full", LMPlusAction::Map3DView { mode: 0 }),
    ("map3dview_balanced", LMPlusAction::Map3DView { mode: 1 }),
    ("map3dview_none", LMPlusAction::Map3DView { mode: 2 }),
    ("switch_account_direct", LMPlusAction::SwitchAccount),
];

/// Resolve either a named id (formation_inf_phalanx) or a parameterized spec
/// (formation:3, map3dview:2, zoom:0.5).
pub fn resolve_named_or_spec(spec: &str) -> Option<LMPlusAction> {
    for (name, action) in &NAMED_ACTIONS {
        if *name == spec {
            return Some(action.clone());
        }
    }
    resolve_hotkey_action(spec)
}

/// Hotkey action strings used by the frontend (`formation:3`, `map3dview:0`, …).
pub fn resolve_hotkey_action(action: &str) -> Option<LMPlusAction> {
    let (kind, arg) = action.split_once(':')?;
    match kind {
        "formation" => {
            let index = arg.parse::<u8>().ok()?;
            if index as usize >= FORMATION_NAMES.len() {
                return None;
            }
            Some(LMPlusAction::Formation { index })
        }
        "map3dview" => {
            let mode = arg.parse::<u8>().ok()?;
            if mode as usize >= MAP3DVIEW_MODE_NAMES.len() {
                return None;
            }
            Some(LMPlusAction::Map3DView { mode })
        }
        "mapzoom" => {
            let value = arg.parse::<f32>().ok()?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return None;
            }
            Some(LMPlusAction::MapZoom { value })
        }
        "switch_account" => Some(LMPlusAction::SwitchAccount),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_formations_resolve() {
        for i in 0..6u8 {
            let a = resolve_hotkey_action(&format!("formation:{}", i));
            assert_eq!(a, Some(LMPlusAction::Formation { index: i }));
        }
        assert_eq!(resolve_hotkey_action("formation:6"), None);
        assert_eq!(resolve_hotkey_action("formation:255"), None);
    }

    #[test]
    fn all_three_map3dview_modes_resolve() {
        for m in 0..3u8 {
            let a = resolve_hotkey_action(&format!("map3dview:{}", m));
            assert_eq!(a, Some(LMPlusAction::Map3DView { mode: m }));
        }
        assert_eq!(resolve_hotkey_action("map3dview:3"), None);
    }

    #[test]
    fn wire_round_trip_matches_agent_protocol() {
        assert_eq!(
            LMPlusAction::Formation { index: 3 }.to_wire(),
            "{\"action\":\"formation\",\"index\":3}"
        );
        assert_eq!(
            LMPlusAction::Map3DView { mode: 2 }.to_wire(),
            "{\"action\":\"map3dview\",\"mode\":2}"
        );
        assert_eq!(
            LMPlusAction::MapZoom { value: 0.5 }.to_wire(),
            "{\"action\":\"zoom\",\"level\":0.5}"
        );
        assert_eq!(
            LMPlusAction::SwitchAccount.to_wire(),
            "{\"action\":\"switch_account\"}"
        );
    }

    #[test]
    fn human_names_cover_every_variant() {
        for i in 0..6u8 {
            assert!(!LMPlusAction::Formation { index: i }.human_name().is_empty());
        }
        for m in 0..3u8 {
            assert!(!LMPlusAction::Map3DView { mode: m }.human_name().is_empty());
        }
        assert_eq!(
            LMPlusAction::Formation { index: 0 }.human_name(),
            "Infantry Phalanx"
        );
        assert_eq!(
            LMPlusAction::Map3DView { mode: 2 }.human_name(),
            "Kingdom Map 3D View: None"
        );
    }

    #[test]
    fn invalid_actions_do_not_resolve() {
        assert_eq!(resolve_hotkey_action("bogus:1"), None);
        assert_eq!(resolve_hotkey_action("formation"), None);
        assert_eq!(resolve_hotkey_action("formation:abc"), None);
        assert_eq!(resolve_hotkey_action("mapzoom:nan"), None);
    }
}
