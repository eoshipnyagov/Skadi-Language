use std::fs;
use std::path::PathBuf;

use crate::project::load_project;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.len() == 1 && args[0] == "--list" {
        println!("available example sets: game, embedded, console, gui");
        return Ok(());
    }

    let project = load_project()?;
    let mut project_type = project.project_type.clone();
    if args.len() == 2 && args[0] == "--type" {
        project_type = super::new_cmd::normalize_type(&args[1])?;
    } else if !args.is_empty() {
        return Err("Usage: skadi examples [--list] [--type <game|embedded|console|gui>]".to_string());
    }

    let examples_dir = project.root.join("examples");
    fs::create_dir_all(&examples_dir)
        .map_err(|e| format!("create {} failed: {e}", examples_dir.display()))?;

    let files = templates_for(&project_type);
    for (name, content) in files {
        let path: PathBuf = examples_dir.join(name);
        if path.exists() {
            continue;
        }
        fs::write(&path, content).map_err(|e| format!("write {} failed: {e}", path.display()))?;
    }

    println!("examples added for type '{}': {}", project_type, examples_dir.display());
    Ok(())
}

fn templates_for(project_type: &str) -> Vec<(&'static str, &'static str)> {
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
