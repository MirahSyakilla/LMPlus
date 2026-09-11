#![windows_subsystem = "windows"]

fn main() {
    // Debug/automation entry points (also available in release builds, but only
    // via explicit CLI flags so normal launches are unaffected):
    //   lmplus.exe --selftest       run the whitelisted test matrix
    //   lmplus.exe --selftest=dry   validate prereqs only, no actions
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest" || a == "--selftest=dry") {
        let dry = args.iter().any(|a| a == "--selftest=dry");
        let results = lmplus_lib::selftest_entry(dry);
        println!("{}", results);
        #[cfg(windows)]
        {
            // Also drop a copy next to the exe for easy retrieval.
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let _ = std::fs::write(dir.join("selftest-results.txt"), &results);
                }
            }
        }
        std::process::exit(0);
    }
    lmplus_lib::run()
}
