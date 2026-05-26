use std::collections::BTreeSet;

use crate::targets::{
    builtin_profiles, candidate_invocations, detect_compiler, os_install_hint, shell_probe_hint,
    target_hint,
};

pub fn run(_args: &[String]) -> Result<(), String> {
    println!("skadi doctor");
    println!();
    println!("Host compiler candidates:");
    let host_dummy_c = std::path::Path::new("dummy.c");
    let host_dummy_out = std::path::Path::new("dummy.out");
    let host_candidates = candidate_invocations("host", host_dummy_c, host_dummy_out)?;
    let mut host_seen: BTreeSet<String> = BTreeSet::new();
    let mut host_ok = false;
    for c in host_candidates {
        if !host_seen.insert(c.program.clone()) {
            continue;
        }
        if detect_compiler(&c.program) {
            host_ok = true;
            println!("  [ok]   {}", c.program);
        } else {
            println!("  [miss] {}", c.program);
        }
    }
    if !host_ok {
        println!("  no host compiler detected");
        println!("  install: {}", os_install_hint());
        println!("  probe: run '<compiler> --version' and '{}'", shell_probe_hint());
    }

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
            println!("        hint: {}", target_hint(profile.triple));
        }
    }
    Ok(())
}
