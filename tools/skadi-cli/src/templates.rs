pub const PROJECT_TYPES: [&str; 4] = ["game", "embedded", "console", "gui"];

pub fn normalize_project_type(input: &str) -> Result<String, String> {
    match input {
        "game" | "embedded" | "console" | "gui" => Ok(input.to_string()),
        _ => Err(format!(
            "unknown project type '{}'. allowed: game, embedded, console, gui",
            input
        )),
    }
}

pub fn manifest_content(name: &str, project_type: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\ntype = \"{}\"\nversion = \"0.1.0\"\nedition = \"v1\"\n\n[build]\nentry = \"src/main.skd\"\n",
        name, project_type
    )
}

pub fn main_template(project_type: &str) -> &'static str {
    match project_type {
        "game" => "new Int frame = 0\nloop {\n    output(frame)\n    frame = frame + 1\n    if frame >= 3 {\n        break\n    }\n}\n",
        "embedded" => "new Int tick = 0\nloop {\n    tick = tick + 1\n    if tick >= 5 {\n        break\n    }\n}\n",
        "gui" => "new Text title = \"Skadi GUI App\"\noutput(title)\n",
        _ => "output(\"Hello from Skadi!\")\n",
    }
}

pub fn example_templates(project_type: &str) -> Vec<(&'static str, &'static str)> {
    match project_type {
        "game" => vec![
            ("10_game_loop.skd", "new Int frame = 0\nloop {\n    output(frame)\n    frame = frame + 1\n    if frame >= 10 {\n        break\n    }\n}\n"),
            ("20_entities.skd", "struct Entity {\n    Int id\n    Text name\n}\n\nnew Entity List entities = []\nentities.push({id = 1, name = \"Player\"})\n"),
        ],
        "embedded" => vec![
            ("10_ticks.skd", "new Int tick = 0\nloop {\n    tick = tick + 1\n    if tick >= 100 {\n        break\n    }\n}\n"),
            ("20_sampling_mock.skd", "new Int List samples = []\nsamples.push(42)\noutput(len(samples))\n"),
        ],
        "gui" => vec![
            ("10_window_mock.skd", "new Text title = \"Skadi GUI\"\noutput(title)\n"),
            ("20_event_mock.skd", "new Text event = \"click\"\nwhen event {\n    is \"click\" {\n        output(\"clicked\")\n    }\n    else {\n        output(\"other\")\n    }\n}\n"),
        ],
        _ => vec![
            ("10_args.skd", "new Text List argv = args()\noutput(len(argv))\n"),
            ("20_read_write.skd", "new Text body = read(\"in.txt\")\nnew Int ok = write(\"out.txt\", body)\noutput(ok)\n"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{example_templates, main_template, manifest_content, normalize_project_type};

    #[test]
    fn normalize_project_type_accepts_known_values() {
        assert_eq!(normalize_project_type("game").expect("ok"), "game");
        assert_eq!(normalize_project_type("embedded").expect("ok"), "embedded");
        assert_eq!(normalize_project_type("console").expect("ok"), "console");
        assert_eq!(normalize_project_type("gui").expect("ok"), "gui");
    }

    #[test]
    fn normalize_project_type_rejects_unknown() {
        let err = normalize_project_type("web").expect_err("must reject");
        assert!(err.contains("unknown project type"));
    }

    #[test]
    fn manifest_contains_type_and_entry() {
        let toml = manifest_content("demo", "game");
        assert!(toml.contains("name = \"demo\""));
        assert!(toml.contains("type = \"game\""));
        assert!(toml.contains("entry = \"src/main.skd\""));
    }

    #[test]
    fn templates_are_non_empty() {
        assert!(!main_template("console").is_empty());
        assert!(!example_templates("game").is_empty());
    }
}
