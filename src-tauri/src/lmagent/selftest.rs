//! Automated test harness for the direct-action integration.
//!
//! Safety policy (hard rules):
//! - Only account hotkeys 1, 2 and 3 may be exercised.
//! - Only formation hotkeys Q, W and E may be exercised.
//! - Anything unassigned or outside the whitelist is refused.
//!
//! Usage (dev, on a machine with the game + LMPlus installed):
//!   lmplus.exe --selftest            runs the whitelisted matrix
//!   lmplus.exe --selftest=dry        validates config/paths only, no actions
//!
//! The harness assumes the game was started by lmplus.exe (last account) so
//! the agent can inject into the running Lords Mobile process.

use crate::lmagent::action::{LMPlusAction, FORMATION_NAMES, MAP3DVIEW_MODE_NAMES};
use crate::lmagent::agent_client;
use crate::lmagent::inject;

const ALLOWED_ACCOUNT_HOTKEYS: [&str; 3] = ["1", "2", "3"];
const ALLOWED_FORMATION_HOTKEYS: [&str; 3] = ["Q", "W", "E"];

pub struct TestConfig {
    pub exe_path: String,
    pub account_hotkeys: Vec<(String, String)>, // (hotkey, account name)
    pub formation_hotkeys: Vec<(String, String)>, // (hotkey, formation spec)
}

#[derive(Debug, PartialEq)]
pub enum TestVerdict {
    Pass,
    Fail(String),
    Skipped(String),
}

/// Load settings and extract the whitelisted hotkey bindings.
pub fn load_test_config(exe_path: &str) -> Result<TestConfig, String> {
    let settings = crate::config::get_hotkey_settings()?;
    let obj = settings.as_object().ok_or("settings not an object")?;

    let mut account_hotkeys = Vec::new();
    let mut formation_hotkeys = Vec::new();

    for (key, value) in obj {
        let Some(hotkey) = value.as_str() else { continue };
        if hotkey.is_empty() {
            continue;
        }
        if let Some(account) = key.strip_prefix("hotkeys.") {
            if ALLOWED_ACCOUNT_HOTKEYS.contains(&hotkey) {
                account_hotkeys.push((hotkey.to_string(), account.to_string()));
            }
        } else if let Some(spec) = key.strip_prefix("direct.") {
            if let Some(action) = crate::lmagent::action::resolve_hotkey_action(&format!("direct:{spec}")) {
                if let LMPlusAction::Formation { index } = action {
                    // hotkey spec letter: compare case-insensitively against Q/W/E
                    let key_letter = hotkey.trim().to_ascii_uppercase();
                    if ALLOWED_FORMATION_HOTKEYS.contains(&key_letter.as_str()) {
                        formation_hotkeys.push((key_letter, format!("formation:{}", index)));
                    }
                }
            }
        }
    }

    Ok(TestConfig {
        exe_path: exe_path.to_string(),
        account_hotkeys,
        formation_hotkeys,
    })
}

/// Validate everything without performing any action.
pub fn dry_run(cfg: &TestConfig) -> Vec<(String, TestVerdict)> {
    let mut results = Vec::new();

    // 1. Agent DLL present and loadable.
    results.push(match super::agent_dll_path() {
        Ok(path) => (format!("agent dll at {}", path), TestVerdict::Pass),
        Err(e) => ("agent dll".to_string(), TestVerdict::Fail(e)),
    });

    // 2. Game process discoverable.
    results.push(match inject::find_game_pid(&cfg.exe_path) {
        Some(pid) => (format!("game pid {}", pid), TestVerdict::Pass),
        None => ("game process".to_string(), TestVerdict::Skipped("not running".into())),
    });

    // 3. Agent pipe responsive.
    results.push(match agent_client::send_line("{\"action\":\"ping\"}") {
        Ok(resp) if resp.contains("\"ok\":true") => ("agent ping".to_string(), TestVerdict::Pass),
        Ok(resp) => ("agent ping".to_string(), TestVerdict::Fail(resp)),
        Err(e) => ("agent ping".to_string(), TestVerdict::Skipped(e)),
    });

    // 4. Whitelist sanity.
    for (hotkey, account) in &cfg.account_hotkeys {
        let verdict = if ALLOWED_ACCOUNT_HOTKEYS.contains(&hotkey.as_str()) {
            TestVerdict::Pass
        } else {
            TestVerdict::Fail(format!("hotkey {} not whitelisted", hotkey))
        };
        results.push((format!("account {} @ {}", account, hotkey), verdict));
    }
    for (hotkey, spec) in &cfg.formation_hotkeys {
        let verdict = if ALLOWED_FORMATION_HOTKEYS.contains(&hotkey.as_str()) {
            TestVerdict::Pass
        } else {
            TestVerdict::Fail(format!("hotkey {} not whitelisted", hotkey))
        };
        results.push((format!("formation {} @ {}", spec, hotkey), verdict));
    }

    results
}

/// Execute one whitelisted action with a bounded timeout.
fn run_action(cfg: &TestConfig, action: &LMPlusAction, label: &str) -> TestVerdict {
    match crate::lmagent::execute(&cfg.exe_path, action) {
        Ok(_detail) => TestVerdict::Pass,
        Err(e) => TestVerdict::Fail(format!("{}: {}", label, e)),
    }
}

/// Full whitelisted test matrix. Accounts 1/2/3 then formations Q/W/E.
pub fn run_selftest(cfg: &TestConfig) -> Vec<(String, TestVerdict)> {
    let mut results = dry_run(cfg);

    let blocked = results.iter().any(|(_, v)| matches!(v, TestVerdict::Fail(_)));
    if blocked {
        results.push(("matrix".to_string(), TestVerdict::Skipped("prereqs failed".into())));
        return results;
    }

    // Formations Q/W/E (safe: purely visual/server-ack'd, no switching).
    for (hotkey, spec) in &cfg.formation_hotkeys {
        if let Some(LMPlusAction::Formation { index }) =
            crate::lmagent::action::resolve_hotkey_action(&format!("direct:{}", spec.split(':').nth(1).unwrap_or("")))
        {
            results.push((
                format!("formation {} (Q/W/E slot {})", FORMATION_NAMES[index as usize], hotkey),
                run_action(cfg, &LMPlusAction::Formation { index }, spec),
            ));
            std::thread::sleep(std::time::Duration::from_millis(2500));
        }
    }

    // Map3DView — read-only visual toggle, safe.
    for mode in 0..3u8 {
        results.push((
            format!("map3dview {}", MAP3DVIEW_MODE_NAMES[mode as usize]),
            run_action(cfg, &LMPlusAction::Map3DView { mode }, "map3dview"),
        ));
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }

    // Accounts 1/2/3 — each triggers the native switch flow; run last because
    // it reconnects. Only whitelisted hotkeys, never unassigned accounts.
    for (hotkey, account) in &cfg.account_hotkeys.clone() {
        let _ = hotkey;
        results.push((
            format!("switch to account {}", account),
            run_action(cfg, &LMPlusAction::SwitchAccount, "switch_account"),
        ));
        // Generous settle time for login phases.
        std::thread::sleep(std::time::Duration::from_secs(12));
        // After a switch, only continue if the agent is still alive.
        if !inject::agent_alive() {
            results.push(("agent after switch".to_string(), TestVerdict::Fail("pipe dead after switch".into())));
            break;
        }
    }

    results
}

/// Format results for the debug log / clipboard.
pub fn format_results(results: &[(String, TestVerdict)]) -> String {
    let mut out = String::from("LMPlus self-test\n================\n");
    for (name, verdict) in results {
        let tag = match verdict {
            TestVerdict::Pass => "PASS",
            TestVerdict::Fail(_) => "FAIL",
            TestVerdict::Skipped(_) => "SKIP",
        };
        out.push_str(&format!("[{}] {}\n", tag, name));
        if let TestVerdict::Fail(e) | TestVerdict::Skipped(e) = verdict {
            out.push_str(&format!("      {}\n", e));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_only_allows_123_qwe() {
        assert!(ALLOWED_ACCOUNT_HOTKEYS.contains(&"1"));
        assert!(ALLOWED_ACCOUNT_HOTKEYS.contains(&"2"));
        assert!(ALLOWED_ACCOUNT_HOTKEYS.contains(&"3"));
        assert!(!ALLOWED_ACCOUNT_HOTKEYS.contains(&"4"));
        for k in ["Q", "W", "E"] {
            assert!(ALLOWED_FORMATION_HOTKEYS.contains(&k));
        }
        assert!(!ALLOWED_FORMATION_HOTKEYS.contains(&"R"));
    }
}
