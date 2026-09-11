//! Direct LMPlus action dispatcher: ensures the agent is injected, sends the
//! action over IPC, and reports the result.

pub mod action;
pub mod agent_client;
pub mod inject;

use action::LMPlusAction;
use std::sync::Mutex;

static ACTION_LOCK: Mutex<()> = Mutex::new(());

/// Execute an LMPlusAction against the running game.
/// Blocks until the agent replies (or errors out).
pub fn execute(exe_path: &str, action: &LMPlusAction) -> Result<String, String> {
    let _guard = ACTION_LOCK.lock().map_err(|e| e.to_string())?;

    let dll_path = agent_dll_path()?;
    inject::ensure_agent(exe_path, &dll_path)?;

    let response = agent_client::send_line(&action.to_wire())?;
    parse_response(&response)
}

fn parse_response(response: &str) -> Result<String, String> {
    if response.contains("\"ok\":true") {
        // Extract "detail" loosely — full JSON parsing is unnecessary here.
        let detail = response
            .split("\"detail\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or("");
        Ok(detail.to_string())
    } else if let Some(err) = response.split("\"error\":\"").nth(1) {
        Err(err
            .split('"')
            .next()
            .unwrap_or("unknown agent error")
            .to_string())
    } else {
        Err(format!("malformed agent response: {}", response))
    }
}

/// Locate lmp_agent.dll next to LMPlus.exe.
fn agent_dll_path() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let dir = exe
        .parent()
        .ok_or("failed to get exe directory")?;
    let path = dir.join(inject::AGENT_DLL_NAME);
    if !path.exists() {
        return Err(format!("agent DLL missing: {}", path.display()));
    }
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ok_response() {
        let r = parse_response("{\"ok\":true,\"detail\":\"formation 3\"}");
        assert_eq!(r.unwrap(), "formation 3");
    }

    #[test]
    fn parses_err_response() {
        let r = parse_response("{\"ok\":false,\"error\":\"formation index out of range\"}");
        assert_eq!(r.unwrap_err(), "formation index out of range");
    }

    #[test]
    fn rejects_malformed_response() {
        assert!(parse_response("garbage").is_err());
        assert!(parse_response("").is_err());
    }
}
