use crate::actions;

const USAGE: &str = "Usage:
  skadi-cli embedded prepare
  skadi-cli embedded build
  skadi-cli embedded flash [--port <port>] [--monitor]
  skadi-cli embedded monitor [--port <port>]";

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("prepare") => {
            if args.len() != 1 {
                return Err(USAGE.to_string());
            }
            let result = actions::prepare_esp_idf().map_err(|error| error.to_string())?;
            println!("ESP-IDF project prepared: {}", result.project_dir.display());
            println!("generated C: {}", result.generated_c.display());
            for warning in result.warnings {
                println!("warning: {}", warning.message);
            }
            Ok(())
        }
        Some("build") => {
            if args.len() != 1 {
                return Err(USAGE.to_string());
            }
            let options = actions::BuildOptions {
                target: "esp32-idf".to_string(),
                cc: None,
            };
            let result = actions::run_build(&options).map_err(|error| error.to_string())?;
            println!("ESP-IDF build ok: {}", result.exe_path.display());
            Ok(())
        }
        Some("flash") => {
            let (port, monitor) = parse_port_and_monitor(&args[1..], true)?;
            let mut commands = vec!["flash".to_string()];
            if monitor {
                commands.push("monitor".to_string());
            }
            let result = actions::run_esp_idf_command(port.as_deref(), &commands)
                .map_err(|error| error.to_string())?;
            println!(
                "ESP-IDF command ok [{}]: {} {}",
                result.status,
                result.invocation.program,
                result.invocation.args.join(" ")
            );
            println!("project: {}", result.prepared.project_dir.display());
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }
            Ok(())
        }
        Some("monitor") => {
            let (port, monitor) = parse_port_and_monitor(&args[1..], false)?;
            if monitor {
                return Err(USAGE.to_string());
            }
            let result = actions::run_esp_idf_command(port.as_deref(), &["monitor".to_string()])
                .map_err(|error| error.to_string())?;
            println!(
                "ESP-IDF monitor ended [{}]: {}",
                result.status,
                result.prepared.project_dir.display()
            );
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

fn parse_port_and_monitor(
    args: &[String],
    allow_monitor: bool,
) -> Result<(Option<String>, bool), String> {
    let mut port = None;
    let mut monitor = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--port" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--port requires a value".to_string());
                };
                port = Some(value.clone());
                index += 2;
            }
            "--monitor" if allow_monitor => {
                monitor = true;
                index += 1;
            }
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok((port, monitor))
}

#[cfg(test)]
mod tests {
    use super::parse_port_and_monitor;

    #[test]
    fn parses_flash_port_and_monitor() {
        let args = vec![
            "--port".to_string(),
            "COM7".to_string(),
            "--monitor".to_string(),
        ];
        let parsed = parse_port_and_monitor(&args, true).expect("parse embedded options");
        assert_eq!(parsed, (Some("COM7".to_string()), true));
    }
}
