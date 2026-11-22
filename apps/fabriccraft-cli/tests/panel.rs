use std::{
    fs, io,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const MODEL: &str = include_str!("../../../examples/planar-panel-200x100.obj");

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
