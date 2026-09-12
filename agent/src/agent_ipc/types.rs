#[derive(Debug, Clone, PartialEq)]
pub enum ActionRequest {
    Ping,
    LogDir(String),
    IsTyping,
    Formation { index: u8 },
    Map3DView { mode: u8 },
    MapZoom { value: f32 },
    SwitchAccount,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionResponse {
    Ok(String),
    Err(String),
}

impl ActionResponse {
    pub fn is_ok_true(&self) -> bool {
        matches!(self, ActionResponse::Ok(d) if d == "true")
    }
}

pub const FORMATION_COUNT: usize = 6; // Infantry/Ranged/Cavalry × Phalanx/Wedge
pub const MAP3DVIEW_MODE_COUNT: usize = 3; // Full / Balanced / None

/// Minimal JSON shim for the flat one-level objects this protocol uses.
/// Deliberately dependency-free so the agent DLL has no external crates.
pub mod serde_json_shim {
    pub struct Value {
        text: String,
    }

    impl Value {
        pub fn get_str(&self, key: &str) -> Option<String> {
            extract(&self.text, key).map(|s| s.trim_matches('"').to_string())
        }
        pub fn get_num(&self, key: &str) -> Option<f64> {
            let raw = extract(&self.text, key)?;
            raw.trim_matches('"').parse::<f64>().ok()
        }
    }

    fn extract(text: &str, key: &str) -> Option<String> {
        let needle = format!("\"{}\"", key);
        let key_pos = text.find(&needle)?;
        let after_colon = text[key_pos + needle.len()..].strip_prefix(':')?;
        let after_colon = after_colon.trim_start();
        if let Some(stripped) = after_colon.strip_prefix('"') {
            let end = stripped.find('"')?;
            Some(stripped[..end].to_string())
        } else {
            let end = after_colon
                .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
                .unwrap_or(after_colon.len());
            Some(after_colon[..end].to_string())
        }
    }

    pub fn parse(input: &str) -> Result<Value, String> {
        let t = input.trim();
        if !t.starts_with('{') || !t.ends_with('}') {
            return Err("not a JSON object".into());
        }
        Ok(Value { text: t.to_string() })
    }
}

use serde_json_shim::Value;

pub fn parse_request(line: &str) -> Result<ActionRequest, String> {
    let v: Value = serde_json_shim::parse(line)?;
    match v.get_str("action").ok_or("missing action")?.as_str() {
        "ping" => Ok(ActionRequest::Ping),
        "logdir" => Ok(ActionRequest::LogDir(v.get_str("dir").unwrap_or_default())),
        "istyping" => Ok(ActionRequest::IsTyping),
        "formation" => {
            let index = v.get_num("index").ok_or("missing index")?;
            if index < 0.0 || index.floor() != index || index as usize >= FORMATION_COUNT {
                return Err(format!(
                    "formation index out of range 0..={}",
                    FORMATION_COUNT - 1
                ));
            }
            Ok(ActionRequest::Formation { index: index as u8 })
        }
        "map3dview" => {
            let mode = v.get_num("mode").ok_or("missing mode")?;
            if mode < 0.0 || mode.floor() != mode || mode as usize >= MAP3DVIEW_MODE_COUNT {
                return Err(format!(
                    "map3dview mode out of range 0..={}",
                    MAP3DVIEW_MODE_COUNT - 1
                ));
            }
            Ok(ActionRequest::Map3DView { mode: mode as u8 })
        }
        "zoom" => {
            let level = v.get_num("level").ok_or("missing level")?;
            if !(0.0..=1.0).contains(&level) || !level.is_finite() {
                return Err("zoom level out of range 0.0..=1.0".into());
            }
            Ok(ActionRequest::MapZoom { value: level as f32 })
        }
        "switch_account" => Ok(ActionRequest::SwitchAccount),
        other => Err(format!("unknown action '{}'", other)),
    }
}

pub fn serialize_response(resp: &ActionResponse) -> String {
    match resp {
        ActionResponse::Ok(detail) => {
            format!("{{\"ok\":true,\"detail\":\"{}\"}}", escape(detail))
        }
        ActionResponse::Err(error) => {
            format!("{{\"ok\":false,\"error\":\"{}\"}}", escape(error))
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ping() {
        assert_eq!(parse_request("{\"action\":\"ping\"}").unwrap(), ActionRequest::Ping);
    }

    #[test]
    fn parses_all_six_formations() {
        for i in 0..6u8 {
            let req = parse_request(&format!("{{\"action\":\"formation\",\"index\":{}}}", i)).unwrap();
            assert_eq!(req, ActionRequest::Formation { index: i });
        }
    }

    #[test]
    fn rejects_formation_out_of_range() {
        assert!(parse_request("{\"action\":\"formation\",\"index\":6}").is_err());
        assert!(parse_request("{\"action\":\"formation\",\"index\":255}").is_err());
        assert!(parse_request("{\"action\":\"formation\",\"index\":-1}").is_err());
    }

    #[test]
    fn parses_all_three_map3dview_modes() {
        for m in 0..3u8 {
            let req = parse_request(&format!("{{\"action\":\"map3dview\",\"mode\":{}}}", m)).unwrap();
            assert_eq!(req, ActionRequest::Map3DView { mode: m });
        }
        assert!(parse_request("{\"action\":\"map3dview\",\"mode\":3}").is_err());
    }

    #[test]
    fn parses_zoom_level() {
        assert_eq!(
            parse_request("{\"action\":\"zoom\",\"level\":0.5}").unwrap(),
            ActionRequest::MapZoom { value: 0.5 }
        );
        assert_eq!(
            parse_request("{\"action\":\"zoom\",\"level\":0.0}").unwrap(),
            ActionRequest::MapZoom { value: 0.0 }
        );
        assert_eq!(
            parse_request("{\"action\":\"zoom\",\"level\":1.0}").unwrap(),
            ActionRequest::MapZoom { value: 1.0 }
        );
        assert!(parse_request("{\"action\":\"zoom\",\"level\":1.5}").is_err());
        assert!(parse_request("{\"action\":\"zoom\",\"level\":-0.1}").is_err());
        assert!(parse_request("{\"action\":\"zoom\",\"value\":12.5}").is_err()); // old key rejected
    }

    #[test]
    fn parses_logdir() {
        assert_eq!(
            parse_request("{\"action\":\"logdir\",\"dir\":\"C:/LMPlus\"}").unwrap(),
            ActionRequest::LogDir("C:/LMPlus".into())
        );
    }

    #[test]
    fn parses_switch_account() {
        assert_eq!(
            parse_request("{\"action\":\"switch_account\"}").unwrap(),
            ActionRequest::SwitchAccount
        );
    }

    #[test]
    fn rejects_unknown_action() {
        assert!(parse_request("{\"action\":\"nuke\"}").is_err());
        assert!(parse_request("{}").is_err());
        assert!(parse_request("not json").is_err());
    }

    #[test]
    fn serializes_responses() {
        assert_eq!(
            serialize_response(&ActionResponse::Ok("pong".into())),
            "{\"ok\":true,\"detail\":\"pong\"}"
        );
        assert_eq!(
            serialize_response(&ActionResponse::Err("bad".into())),
            "{\"ok\":false,\"error\":\"bad\"}"
        );
    }
}
