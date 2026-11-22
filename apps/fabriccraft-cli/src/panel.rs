use std::{
    error::Error,
    fmt::Write as _,
    fs::{self, File},
    io::{Cursor, Read, Write as _},
    path::Path,
};

use fabriccraft_core::{
    geometry::{
        Face, FaceId, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId, Vertex, VertexId,
    },
    planar::PlanarPanel,
};

const MAX_OBJ_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TRIANGLES: usize = 100_000;
const MARGIN_MM: f64 = 1.;

pub fn export(input: &Path, unit: Option<&str>, output: &Path) -> Result<(), Box<dyn Error>> {
    let unit = match unit {
        Some("mm") => LengthUnit::Millimetre,
        Some("cm") => LengthUnit::Centimetre,
        Some("m") => LengthUnit::Metre,
        Some("in") => LengthUnit::Inch,
        _ => return Err("specify the OBJ length unit as mm, cm, m, or in".into()),
    };
    if !has_extension(input, "obj") || !has_extension(output, "svg") {
        return Err("use an .obj input path and an .svg output path".into());
    }

    let surface = import_obj(input, unit)?;
    let panel = PlanarPanel::from_surface(&surface)?;
    let svg = render(&panel)?;
    let mut file = File::create_new(output)
        .map_err(|err| format!("could not create {}: {err}", output.display()))?;
    if let Err(error) = file.write_all(svg.as_bytes()) {
        drop(file);
        let message = format!("could not finish writing {}: {error}", output.display());
        return match fs::remove_file(output) {
            Ok(()) => Err(message.into()),
            Err(cleanup) => {
                Err(format!("{message}. Could not remove the partial file ({cleanup})").into())
            }
        };
    }
    let [width, height] = panel.size_mm();
    println!(
        "Panel {width:.3} × {height:.3} mm. Saved {}",
        output.display()
    );
    Ok(())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(expected))
}

fn import_obj(path: &Path, unit: LengthUnit) -> Result<Surface, Box<dyn Error>> {
    let file =
        File::open(path).map_err(|err| format!("could not open {}: {err}", path.display()))?;
    let mut content = String::new();
    file.take(MAX_OBJ_BYTES + 1).read_to_string(&mut content)?;
    if content.len() as u64 > MAX_OBJ_BYTES {
        return Err("OBJ exceeds the 8 MiB input limit".into());
    }

    let mut geometry = String::with_capacity(content.len());
    let mut triangle_count = 0;
    for line in content.trim_start_matches('\u{feff}').lines() {
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        let mut words = line.split_whitespace();
        match words.next() {
            Some("f") => {
                if words.count() != 3 {
                    return Err(
                        "OBJ faces must be triangles. Enable triangulation when exporting from CAD"
                            .into(),
                    );
                }
                triangle_count += 1;
                if triangle_count > MAX_TRIANGLES {
                    return Err("OBJ exceeds the 100,000-triangle limit".into());
                }
            }
            Some("l" | "p" | "curv" | "curv2" | "surf") => {
                return Err("export one tessellated surface as a triangle mesh".into());
            }
            _ => {}
        }
        geometry.push_str(line);
        geometry.push('\n');
    }

    // Preserve position indices shared across UV/normal seams. Read only the
    // OBJ file, with an empty material library supplied in memory.
    let options = tobj::LoadOptions::default();
    let (models, _) = tobj::load_obj_buf(&mut Cursor::new(geometry), &options, |_| {
        Ok((Vec::new(), Default::default()))
    })
    .map_err(|err| format!("could not parse OBJ: {err}"))?;
    let mut meshes = models
        .into_iter()
        .filter(|model| !model.mesh.indices.is_empty());
    let mesh = meshes.next().ok_or("OBJ contains no triangle faces")?.mesh;
    if meshes.next().is_some() {
        return Err("export one panel mesh without separate OBJ objects or groups".into());
    }
    let (positions, partial_position) = mesh.positions.as_chunks::<3>();
    let (indices, partial_face) = mesh.indices.as_chunks::<3>();
    if !partial_position.is_empty() || !partial_face.is_empty() {
        return Err("OBJ parser returned incomplete triangle data".into());
    }
    let vertices = positions
        .iter()
        .enumerate()
        .map(|(id, &position)| Vertex {
            id: VertexId(id as u64),
            position,
        })
        .collect();
    let faces = indices
        .iter()
        .enumerate()
        .map(|(id, &corners)| Face {
            id: FaceId(id as u64),
            vertices: corners.map(|index| VertexId(u64::from(index))),
        })
        .collect();
    Ok(Surface::import(
        SurfaceId(1),
        Provenance {
            source: path.to_string_lossy().into_owned(),
            unit,
            frame: ImportFrame::IDENTITY,
        },
        vertices,
        faces,
    )?)
}

fn render(panel: &PlanarPanel) -> Result<String, std::fmt::Error> {
    let [width, height] = panel.size_mm().map(|size| size + 2. * MARGIN_MM);
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}mm" height="{height}mm" viewBox="-{MARGIN_MM} -{MARGIN_MM} {width} {height}">"#
    );
    let panel_height = panel.size_mm()[1];
    write!(
        svg,
        "\n  <polygon fill=\"none\" stroke=\"black\" stroke-width=\"0.2\" transform=\"translate(0 {panel_height}) scale(1 -1)\" points=\""
    )?;
    for (index, [x, y]) in panel.outline_mm().iter().enumerate() {
        if index != 0 {
            svg.push(' ');
        }
        write!(svg, "{x},{y}")?;
    }
    svg.push_str("\"/>\n</svg>\n");
    Ok(svg)
}
