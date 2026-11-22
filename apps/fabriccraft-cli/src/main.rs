mod panel;

use std::{env, path::Path, process::ExitCode};

const USAGE: &str = "Usage:\n  fabriccraft-cli surfaces <mesh.obj> <mm|cm|m|in>\n  fabriccraft-cli seam-vertices <mesh.obj> <mm|cm|m|in> [--exclude-surfaces NUMBER,...]\n  fabriccraft-cli panel <mesh.obj> <mm|cm|m|in> <pattern.svg> [--surface NUMBER] [--fabric-mm WIDTH LENGTH]\n  fabriccraft-cli develop <mesh.obj> <mm|cm|m|in> <pattern.svg> [--exclude-surfaces NUMBER,...] [--open-tube | --seam VERTEX,VERTEX,...] [--fabric-mm WIDTH LENGTH]";

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
        [command, mesh, unit] if command == "surfaces" => {
            panel::list_surfaces(Path::new(mesh), unit.to_str())
        }
        [command, mesh, unit, options @ ..] if command == "seam-vertices" => {
            panel::list_seam_vertices(Path::new(mesh), unit.to_str(), options)
        }
        [command, mesh, unit, svg, options @ ..] if command == "panel" => panel::export(
            Path::new(mesh),
            unit.to_str(),
            Path::new(svg),
            options,
            panel::Mode::Planar,
        ),
        [command, mesh, unit, svg, options @ ..] if command == "develop" => panel::export(
            Path::new(mesh),
            unit.to_str(),
            Path::new(svg),
            options,
            panel::Mode::Develop,
        ),
        _ => Err(USAGE.into()),
    };
    if let Err(error) = result {
        eprintln!("FabricCraft: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
