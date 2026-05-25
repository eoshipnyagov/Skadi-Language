use std::collections::BTreeSet;

use crate::targets::{builtin_profiles, candidate_invocations, detect_compiler};

pub fn run(_args: &[String]) -> Result<(), String> {
    println!("skadi doctor");
    println!();
    println!("Target toolchain availability:");
    for profile in builtin_profiles() {
        let dummy_c = std::path::Path::new("dummy.c");
        let dummy_out = std::path::Path::new("dummy.out");
        let candidates = candidate_invocations(profile.triple, dummy_c, dummy_out)?;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut ok = false;
        for c in candidates {
            if !seen.insert(c.program.clone()) {
                continue;
            }
            if detect_compiler(&c.program) {
                ok = true;
                println!("  [ok]   {} -> {}", profile.triple, c.program);
            } else {
                println!("  [miss] {} -> {}", profile.triple, c.program);
            }
        }
        if !ok {
            println!("        no available compiler found for {}", profile.triple);
        }
    }
    Ok(())
}
