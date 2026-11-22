use std::{
    error::Error,
    ffi::OsString,
    fmt::Write as _,
    fs::{self, File},
    io::{Cursor, Read, Write as _},
    path::Path,
};

use fabriccraft_core::{
    develop::DevelopedPanel,
    geometry::{
        Face, FaceId, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId, Vertex, VertexId,
    },
    planar::{PlanarPanel, PlanarPatch, planar_patches},
};

const MAX_OBJ_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TRIANGLES: usize = 100_000;
const MARGIN_MM: f64 = 1.;

pub enum Mode {
    Planar,
    Develop,
}

#[derive(Default)]
struct Options {
    surface: Option<usize>,
    fabric: Option<[f64; 2]>,
}

impl Options {
    fn parse(mut arguments: &[OsString]) -> Result<Self, Box<dyn Error>> {
        let mut options = Self::default();
        while !arguments.is_empty() {
            match arguments {
                [flag, value, rest @ ..] if flag == "--surface" && options.surface.is_none() => {
                    options.surface = Some(
                        value
                            .to_str()
                            .and_then(|value| value.parse::<usize>().ok())
                            .filter(|&value| value != 0)
                            .ok_or("--surface requires a positive surface number")?,
                    );
                    arguments = rest;
                }
                [flag, width, length, rest @ ..]
                    if flag == "--fabric-mm" && options.fabric.is_none() =>
                {
                    let [Some(width), Some(length)] = [width, length].map(|value| {
                        value
                            .to_str()
                            .and_then(|value| value.parse::<f64>().ok())
                            .filter(|value| value.is_finite() && *value > 0.)
                    }) else {
                        return Err(
                            "--fabric-mm requires positive, finite dimensions: WIDTH LENGTH".into(),
                        );
                    };
                    options.fabric = Some([width, length]);
                    arguments = rest;
                }
                _ => return Err(crate::USAGE.into()),
            }
        }
        Ok(options)
    }
}

pub fn list_surfaces(path: &Path, unit: Option<&str>) -> Result<(), Box<dyn Error>> {
    let meshes = import_obj(path, parse_unit(unit)?)?;
    let patches = model_patches(&meshes)?;
    println!("{} flat mesh patches", patches.len());
    for (index, (name, patch)) in patches.iter().enumerate() {
        let [x, y, z] = patch.center_mm();
        let [nx, ny, nz] = patch.normal().map(|n| if n == 0. { 0. } else { n });
        println!(
            "{}  {name}  {} triangles  normal ({nx:.3}, {ny:.3}, {nz:.3})  center ({x:.3}, {y:.3}, {z:.3}) mm",
            index + 1,
            patch.triangle_count(),
        );
    }
    Ok(())
}

pub fn export(
    input: &Path,
    unit: Option<&str>,
    output: &Path,
    arguments: &[OsString],
    mode: Mode,
) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(arguments)?;
    if !has_extension(output, "svg") {
        return Err("use an .svg output path".into());
    }
    let meshes = import_obj(input, parse_unit(unit)?)?;
    let mut development_errors = None;
    let panel = if matches!(mode, Mode::Develop) {
        let [mesh] = meshes.as_slice() else {
            return Err("develop expects one OBJ object or group with a prepared seam cut".into());
        };
        if options.surface.is_some() {
            return Err("export the pre-cut patch as one OBJ mesh for develop".into());
        }
        let development = DevelopedPanel::from_surface(&mesh.surface)?;
        development_errors = Some((
            development.max_edge_error_mm(),
            development.max_relative_area_error(),
        ));
        development.into_panel()
    } else if let Some(number) = options.surface {
        let patches = model_patches(&meshes)?;
        let (_, patch) = patches.get(number - 1).ok_or_else(|| {
            format!(
                "surface {number} is out of range (1–{}). Run the surfaces command to list choices",
                patches.len()
            )
        })?;
        patch.panel()?
    } else {
        let [mesh] = meshes.as_slice() else {
            return Err("run the surfaces command and choose --surface NUMBER for an OBJ with separate objects or groups".into());
        };
        PlanarPanel::from_surface(&mesh.surface).map_err(|error| {
            format!(
                "{error}. For a whole model, run the surfaces command and choose --surface NUMBER"
            )
        })?
    };
    let fabric = options
        .fabric
        .map(|stock| FabricFit::check(panel.size_mm(), stock))
        .transpose()?;
    let svg = render(&panel, fabric)?;
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
    if let Some((edge, area)) = development_errors {
        println!("Max edge error {edge:.3e} mm. Max relative face-area error {area:.3e}.");
    }
    if let Some(fabric) = fabric {
        println!("{}", fabric.description());
    }
    println!(
        "Panel {width:.3} × {height:.3} mm. Saved {}",
        output.display()
    );
    Ok(())
}

fn parse_unit(unit: Option<&str>) -> Result<LengthUnit, &'static str> {
    match unit {
        Some("mm") => Ok(LengthUnit::Millimetre),
        Some("cm") => Ok(LengthUnit::Centimetre),
        Some("m") => Ok(LengthUnit::Metre),
        Some("in") => Ok(LengthUnit::Inch),
        _ => Err("specify the OBJ length unit as mm, cm, m, or in"),
    }
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(expected))
}

struct ObjMesh {
    name: String,
    surface: Surface,
}

fn model_patches(meshes: &[ObjMesh]) -> Result<Vec<(&str, PlanarPatch<'_>)>, Box<dyn Error>> {
    let mut result = Vec::new();
    for mesh in meshes {
        let patches =
            planar_patches(&mesh.surface).map_err(|error| format!("{}: {error}", mesh.name))?;
        result.extend(patches.into_iter().map(|patch| (mesh.name.as_str(), patch)));
    }
    Ok(result)
}

fn import_obj(path: &Path, unit: LengthUnit) -> Result<Vec<ObjMesh>, Box<dyn Error>> {
    if !has_extension(path, "obj") {
        return Err("use a triangulated .obj input path".into());
    }
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
                return Err("export the reference model as a triangle mesh".into());
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
    let meshes: Vec<_> = models
        .into_iter()
        .filter(|model| !model.mesh.indices.is_empty())
        .enumerate()
        .map(|(index, model)| obj_surface(path, unit, index, model))
        .collect::<Result<_, _>>()?;
    if meshes.is_empty() {
        return Err("OBJ contains no triangle faces".into());
    }
    Ok(meshes)
}

fn obj_surface(
    path: &Path,
    unit: LengthUnit,
    index: usize,
    model: tobj::Model,
) -> Result<ObjMesh, Box<dyn Error>> {
    let tobj::Model { mesh, mut name } = model;
    name.retain(|character| !character.is_control());
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
    let surface = Surface::import(
        SurfaceId(index as u64 + 1),
        Provenance {
            source: path.to_string_lossy().into_owned(),
            unit,
            frame: ImportFrame::IDENTITY,
        },
        vertices,
        faces,
    )
    .map_err(|error| format!("{name}: {error}"))?;
    Ok(ObjMesh { name, surface })
}

#[derive(Clone, Copy)]
struct FabricFit {
    width_mm: f64,
    length_mm: f64,
    quarter_turn: bool,
}

impl FabricFit {
    fn check([width, height]: [f64; 2], [width_mm, length_mm]: [f64; 2]) -> Result<Self, String> {
        // Allow only arithmetic rounding, not a physical cutting tolerance.
        let within = |needed: f64, available: f64| {
            needed <= available || needed - available <= needed * (16. * f64::EPSILON)
        };
        let as_drawn = within(width, width_mm) && within(height, length_mm);
        let quarter_turn = within(height, width_mm) && within(width, length_mm);
        if !as_drawn && !quarter_turn {
            return Err(format!(
                "Panel {width:.3} × {height:.3} mm exceeds the usable fabric ({width_mm} mm wide by {length_mm} mm long) at 0° or 90°"
            ));
        }
        Ok(Self {
            width_mm,
            length_mm,
            quarter_turn: !as_drawn,
        })
    }

    fn description(self) -> String {
        let Self {
            width_mm,
            length_mm,
            quarter_turn,
        } = self;
        let outcome = if quarter_turn {
            "fits with a 90° turn. Check the fabric grain before cutting."
        } else {
            "fits as drawn."
        };
        format!("Usable fabric {width_mm} mm wide by {length_mm} mm long. The outline {outcome}")
    }
}

fn render(panel: &PlanarPanel, fabric: Option<FabricFit>) -> Result<String, std::fmt::Error> {
    let [width, height] = panel.size_mm().map(|size| size + 2. * MARGIN_MM);
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}mm" height="{height}mm" viewBox="-{MARGIN_MM} -{MARGIN_MM} {width} {height}">"#
    );
    if let Some(fabric) = fabric {
        write!(svg, "\n  <desc>{}</desc>", fabric.description())?;
    }
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
