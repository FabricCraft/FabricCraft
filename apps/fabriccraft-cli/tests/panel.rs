use std::{
    fs, io,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const MODEL: &str = include_str!("../../../examples/planar-panel-200x100.obj");
const BOX: &str = include_str!("../../../examples/reference-box.obj");

struct Files {
    input: PathBuf,
    output: PathBuf,
}

impl Files {
    fn new() -> io::Result<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "fabriccraft-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir)?;
        Ok(Self {
            input: dir.join("test panel.obj"),
            output: dir.join("test pattern.svg"),
        })
    }

    fn panel(&self, unit: &str) -> io::Result<Output> {
        Command::new(env!("CARGO_BIN_EXE_fabriccraft-cli"))
            .arg("panel")
            .arg(&self.input)
            .arg(unit)
            .arg(&self.output)
            .output()
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.output);
        let _ = fs::remove_file(&self.input);
        let _ = fs::remove_dir(self.input.parent().unwrap());
    }
}

#[test]
fn writes_millimetre_svg_without_loading_materials_or_overwriting() -> io::Result<()> {
    let files = Files::new()?;
    // The material path refers to a file that doesn't exist.
    fs::write(&files.input, format!("mtllib missing.mtl\n{MODEL}"))?;
    let result = files.panel("cm")?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("2000.000 × 1000.000 mm"));
    let original = fs::read_to_string(&files.output)?;
    assert!(original.contains(r#"width="2002mm" height="1002mm" viewBox="-1 -1 2002 1002""#));
    assert!(original.contains(r#"points="0,0 2000,0 2000,1000 0,1000""#));
    let retry = files.panel("mm")?;
    assert!(!retry.status.success());
    assert_eq!(fs::read_to_string(&files.output)?, original);
    Ok(())
}

#[test]
fn rejects_wrong_units_and_polygons_before_creating_output() -> io::Result<()> {
    let files = Files::new()?;
    fs::write(&files.input, MODEL)?;
    let unit = files.panel("feet")?;
    assert!(!unit.status.success());
    assert!(String::from_utf8_lossy(&unit.stderr).contains("specify the OBJ length unit"));
    assert!(!files.output.exists());

    let quad = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";
    fs::write(&files.input, quad)?;
    let polygon = files.panel("mm")?;
    assert!(!polygon.status.success());
    assert!(String::from_utf8_lossy(&polygon.stderr).contains("OBJ faces must be triangles"));
    assert!(!files.output.exists());
    Ok(())
}

#[test]
fn lists_and_exports_one_surface_of_the_whole_model() -> io::Result<()> {
    let files = Files::new()?;
    fs::write(&files.input, BOX)?;
    let binary = env!("CARGO_BIN_EXE_fabriccraft-cli");
    let list = Command::new(binary)
        .arg("surfaces")
        .arg(&files.input)
        .arg("mm")
        .output()?;
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let list = String::from_utf8_lossy(&list.stdout);
    assert!(list.contains("6 flat mesh patches"));
    assert!(list.contains("1  reference_box  2 triangles  normal (0.000, 0.000, 1.000)"));

    let old_usage = files.panel("mm")?;
    assert!(!old_usage.status.success());
    assert!(!files.output.exists());
    let selected = Command::new(binary)
        .arg("panel")
        .arg(&files.input)
        .arg("mm")
        .arg(&files.output)
        .args(["--surface", "1"])
        .output()?;
    assert!(
        selected.status.success(),
        "{}",
        String::from_utf8_lossy(&selected.stderr)
    );
    assert!(String::from_utf8_lossy(&selected.stdout).contains("200.000 × 100.000 mm"));
    let svg = fs::read_to_string(&files.output)?;
    assert!(svg.contains(r#"width="202mm" height="102mm""#));
    Ok(())
}

#[test]
fn checks_fabric_width_and_length_without_rotating_the_outline() -> io::Result<()> {
    let files = Files::new()?;
    fs::write(&files.input, MODEL)?;
    let run = |width, length| {
        Command::new(env!("CARGO_BIN_EXE_fabriccraft-cli"))
            .arg("panel")
            .arg(&files.input)
            .arg("mm")
            .arg(&files.output)
            .args(["--fabric-mm", width, length])
            .output()
    };

    let too_small = run("90", "210")?;
    assert!(!too_small.status.success());
    assert!(String::from_utf8_lossy(&too_small.stderr).contains("0° or 90°"));
    assert!(!files.output.exists());
    let invalid = run("NaN", "210")?;
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("positive, finite dimensions"));
    assert!(!files.output.exists());

    let turn = run("100", "200")?;
    assert!(
        turn.status.success(),
        "{}",
        String::from_utf8_lossy(&turn.stderr)
    );
    assert!(String::from_utf8_lossy(&turn.stdout).contains("fits with a 90° turn"));
    let svg = fs::read_to_string(&files.output)?;
    assert!(svg.contains(r#"width="202mm" height="102mm""#));
    assert!(svg.contains("Usable fabric 100 mm wide by 200 mm long"));

    fs::remove_file(&files.output)?;
    let as_drawn = run("200", "100")?;
    assert!(as_drawn.status.success());
    assert!(String::from_utf8_lossy(&as_drawn.stdout).contains("fits as drawn"));
    Ok(())
}

#[test]
fn numbers_patches_across_obj_groups_and_checks_the_selected_number() -> io::Result<()> {
    let files = Files::new()?;
    fs::write(
        &files.input,
        format!("{BOX}\ng rear_flap\nv 300 0 0\nv 310 0 0\nv 310 5 0\nf 9 10 11\n"),
    )?;
    let binary = env!("CARGO_BIN_EXE_fabriccraft-cli");
    let list = Command::new(binary)
        .arg("surfaces")
        .arg(&files.input)
        .arg("mm")
        .output()?;
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains("7  rear_flap  1 triangles"));
    let run = |number| {
        Command::new(binary)
            .arg("panel")
            .arg(&files.input)
            .arg("mm")
            .arg(&files.output)
            .args(["--surface", number])
            .output()
    };
    let missing = run("8")?;
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("surface 8 is out of range"));
    assert!(!files.output.exists());
    let result = run("7")?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(files.output.exists());
    Ok(())
}
