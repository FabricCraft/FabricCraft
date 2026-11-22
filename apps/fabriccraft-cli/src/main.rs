mod panel;

use std::{env, path::Path, process::ExitCode};

const USAGE: &str = "Usage: fabriccraft-cli panel <mesh.obj> <mm|cm|m|in> <pattern.svg>";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [] => {
            let version = fabriccraft_core::schema_version();
            println!("FabricCraft pre-alpha (schema {version})");
            Ok(())
        }
        [help] if help == "--help" || help == "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        [command, mesh, unit, svg] if command == "panel" => {
            panel::export(Path::new(mesh), unit.to_str(), Path::new(svg))
        }
        _ => Err(USAGE.into()),
    };
    if let Err(error) = result {
        eprintln!("FabricCraft: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
