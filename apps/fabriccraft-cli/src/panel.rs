use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::{OsStr, OsString},
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
    tube::TubeDevelopment,
};

const MAX_OBJ_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TRIANGLES: usize = 100_000;
const MARGIN_MM: f64 = 1.;

pub enum Mode {
    Planar,
    Develop,
}

enum SeamChoice {
    Shortest,
    Selected(Vec<VertexId>),
}

#[derive(Default)]
struct Options {
    surface: Option<usize>,
    fabric: Option<[f64; 2]>,
    seam: Option<SeamChoice>,
    excluded_surfaces: Vec<usize>,
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
                [flag, rest @ ..] if flag == "--open-tube" && options.seam.is_none() => {
                    options.seam = Some(SeamChoice::Shortest);
                    arguments = rest;
                }
                [flag, vertices, rest @ ..] if flag == "--seam" && options.seam.is_none() => {
                    let path = selection_numbers(vertices).filter(|path| path.len() >= 2)
                      .ok_or("--seam requires a comma-separated path of two or more positive vertex numbers")?;
                    options.seam = Some(SeamChoice::Selected(
                        path.into_iter().map(|n| VertexId((n - 1) as u64)).collect(),
                    ));
                    arguments = rest;
                }
                [flag, surfaces, rest @ ..]
                    if flag == "--exclude-surfaces" && options.excluded_surfaces.is_empty() =>
                {
                    options.excluded_surfaces = selection_numbers(surfaces)
                        .ok_or("--exclude-surfaces requires a comma-separated list of positive surface numbers")?;
                    arguments = rest;
                }
                _ => return Err(crate::USAGE.into()),
            }
        }
        Ok(options)
    }
}

fn selection_numbers(value: &OsStr) -> Option<Vec<usize>> {
    value
        .to_str()?
        .split(',')
        .map(|part| part.trim().parse().ok().filter(|&number| number != 0))
        .collect()
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

pub fn list_seam_vertices(
    path: &Path,
    unit: Option<&str>,
    arguments: &[OsString],
) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(arguments)?;
    if options.surface.is_some() || options.fabric.is_some() || options.seam.is_some() {
        return Err("seam-vertices accepts only --exclude-surfaces".into());
    }
    let meshes = import_obj(path, parse_unit(unit)?)?;
    let [mesh] = meshes.as_slice() else {
        return Err("export one OBJ object or group for seam selection".into());
    };
    let selected = select_coverage(mesh, &options.excluded_surfaces)?;
    let record = selected.record();
    let mut edges = BTreeMap::<VertexId, BTreeSet<VertexId>>::new();
    let mut boundary = BTreeSet::new();
    for face in &record.faces {
        let [a, b, c] = face.vertices;
        for (from, to) in [(a, b), (b, c), (c, a)] {
            edges.entry(from).or_default().insert(to);
            if !boundary.remove(&(to, from)) {
                boundary.insert((from, to));
            }
        }
    }
    let rim_vertices: BTreeSet<_> = boundary.iter().flat_map(|&(a, b)| [a, b]).collect();
    for (a, b) in boundary {
        edges.entry(b).or_default().insert(a);
    }
    for v in &record.vertices_mm {
        let id = v.id;
        let [x, y, z] = v.position;
        let location = if rim_vertices.contains(&id) {
            "rim"
        } else {
            "interior"
        };
        let linked = edges
            .get(&id)
            .into_iter()
            .flatten()
            .map(|id| (id.0 + 1).to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{}  {location}  ({x:.3}, {y:.3}, {z:.3}) mm  edges {linked}",
            id.0 + 1
        );
    }
    Ok(())
}

enum PanelOutput {
    Flat(PlanarPanel),
    Developed(DevelopedPanel),
    Tube(TubeDevelopment),
}

fn select_coverage<'a>(
    mesh: &'a ObjMesh,
    excluded_surfaces: &[usize],
) -> Result<Cow<'a, Surface>, Box<dyn Error>> {
    if excluded_surfaces.is_empty() {
        return Ok(Cow::Borrowed(&mesh.surface));
    }
    let patches = planar_patches(&mesh.surface)?;
    let mut excluded = BTreeSet::new();
    for &number in excluded_surfaces {
        let patch = patches.get(number - 1).ok_or_else(|| {
            format!(
                "surface {number} is out of range (1–{}). Run the surfaces command to list choices",
                patches.len()
            )
        })?;
        excluded.extend(patch.face_ids());
    }
    let selected = mesh
        .surface
        .select_faces(|id| !excluded.contains(&id))
        .map_err(|error| format!("could not select coverage: {error}"))?;
    Ok(Cow::Owned(selected))
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
    if matches!(mode, Mode::Planar)
        && (options.seam.is_some() || !options.excluded_surfaces.is_empty())
    {
        return Err(
            "--seam, --open-tube and --exclude-surfaces are options for the develop command".into(),
        );
    }
    let meshes = import_obj(input, parse_unit(unit)?)?;
    let result = if matches!(mode, Mode::Develop) {
        let [mesh] = meshes.as_slice() else {
            return Err("develop expects one OBJ object or group".into());
        };
        if options.surface.is_some() {
            return Err(
                "use --exclude-surfaces to leave planar faces out of the developed panel".into(),
            );
        }
        let selected = select_coverage(mesh, &options.excluded_surfaces)?;
        match &options.seam {
            None => PanelOutput::Developed(DevelopedPanel::from_surface(&selected)?),
            Some(choice) => {
                let manual = match choice {
                    SeamChoice::Shortest => None,
                    SeamChoice::Selected(path) => Some(path.as_slice()),
                };
                PanelOutput::Tube(TubeDevelopment::from_surface(&selected, manual)?)
            }
        }
    } else if let Some(number) = options.surface {
        let patches = model_patches(&meshes)?;
        let (_, patch) = patches.get(number - 1).ok_or_else(|| {
            format!(
                "surface {number} is out of range (1–{}). Run the surfaces command to list choices",
                patches.len()
            )
        })?;
        PanelOutput::Flat(patch.panel()?)
    } else {
        let [mesh] = meshes.as_slice() else {
            return Err("run the surfaces command and choose --surface NUMBER for an OBJ with separate objects or groups".into());
        };
        PanelOutput::Flat(PlanarPanel::from_surface(&mesh.surface).map_err(|error| {
            format!(
                "{error}. For a whole model, run the surfaces command and choose --surface NUMBER"
            )
        })?)
    };
    let (panel, development, seam) = match &result {
        PanelOutput::Flat(panel) => (panel, None, None),
        PanelOutput::Developed(development) => (development.panel(), Some(development), None),
        PanelOutput::Tube(tube) => (
            tube.development().panel(),
            Some(tube.development()),
            Some(tube),
        ),
    };
    let seam_summary = seam.map(describe_seam);
    let fabric = options
        .fabric
        .map(|stock| FabricFit::check(panel.size_mm(), stock))
        .transpose()?;
    let svg = render(panel, fabric, seam.zip(seam_summary.as_deref()))?;
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
    if let Some(development) = development {
        let edge = development.max_edge_error_mm();
        let area = development.max_relative_area_error();
        println!("Max edge error {edge:.3e} mm. Max relative face-area error {area:.3e}.");
    }
    if let Some(summary) = seam_summary {
        println!("{summary}");
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

fn describe_seam(seam: &TubeDevelopment) -> String {
    let length = seam.source_length_mm();
    let error = seam.max_paired_edge_error_mm();
    let route = seam
        .source_path()
        .iter()
        .map(|id| (id.0 + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "Seam {route}. Source length {length:.3} mm. Maximum paired-edge mismatch {error:.3e} mm."
    )
}

fn render(
    panel: &PlanarPanel,
    fabric: Option<FabricFit>,
    seam: Option<(&TubeDevelopment, &str)>,
) -> Result<String, std::fmt::Error> {
    let [width, height] = panel.size_mm().map(|size| size + 2. * MARGIN_MM);
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}mm" height="{height}mm" viewBox="-{MARGIN_MM} -{MARGIN_MM} {width} {height}">"#
    );
    if fabric.is_some() || seam.is_some() {
        svg.push_str("\n  <desc>");
        if let Some(fabric) = fabric {
            write!(svg, "{}", fabric.description())?;
        }
        if let Some((_, summary)) = seam {
            if fabric.is_some() {
                svg.push(' ');
            }
            svg.push_str(summary);
        }
        svg.push_str("</desc>");
    }
    let panel_height = panel.size_mm()[1];
    write!(
        svg,
        "\n  <polygon fill=\"none\" stroke=\"black\" stroke-width=\"0.2\" transform=\"translate(0 {panel_height}) scale(1 -1)\" points=\""
    )?;
    write_points(&mut svg, panel.outline_mm())?;
    svg.push_str("\"/>");
    if let Some((seam, _)) = seam {
        for (name, points) in ["seam-a", "seam-b"].into_iter().zip(seam.banks_mm()) {
            write!(
                svg,
                "\n  <polyline id=\"{name}\" fill=\"none\" stroke=\"#087e8b\" stroke-width=\"0.35\" stroke-dasharray=\"2 1\" transform=\"translate(0 {panel_height}) scale(1 -1)\" points=\""
            )?;
            write_points(&mut svg, points)?;
            svg.push_str("\"/>");
        }
    }
    svg.push_str("\n</svg>\n");
    Ok(svg)
}

fn write_points(svg: &mut String, points: &[[f64; 2]]) -> Result<(), std::fmt::Error> {
    for (index, [x, y]) in points.iter().enumerate() {
        if index != 0 {
            svg.push(' ');
        }
        write!(svg, "{x},{y}")?;
    }
    Ok(())
}
