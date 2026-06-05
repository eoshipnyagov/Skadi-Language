use crate::actions;

pub fn run(_args: &[String]) -> Result<(), String> {
    let report = actions::run_doctor().map_err(|e| e.to_string())?;
    println!("skadi doctor");
    println!();
    println!("Host compiler candidates:");
    for c in &report.host_candidates {
        if c.available {
            println!("  [ok]   {}", c.program);
        } else {
            println!("  [miss] {}", c.program);
        }
    }
    if !report.host_ready {
        println!("  no host compiler detected");
        println!("  install: {}", report.host_install_hint);
        println!(
            "  probe: run '<compiler> --version' and '{}'",
            report.shell_probe_hint
        );
    } else {
        println!("  host toolchain status: ready");
    }

    println!();
    println!("Target toolchain availability:");
    for profile in &report.targets {
        for c in &profile.statuses {
            if c.available {
                println!("  [ok]   {} -> {}", profile.triple, c.program);
            } else {
                println!("  [miss] {} -> {}", profile.triple, c.program);
            }
        }
        if !profile.ready {
            println!("        no available compiler found for {}", profile.triple);
            println!("        hint: {}", profile.hint);
        } else {
            println!("        target status: ready for {}", profile.triple);
        }
    }
    Ok(())
}
