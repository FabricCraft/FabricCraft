use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText};
use fabriccraft_cli::panel::{ObjMesh, import_obj, parse_obj, save_panel_svg};
use fabriccraft_core::{
    develop::DevelopedPanel,
    geometry::{Axis, LengthUnit, Surface},
    skin::{build_skin, suggest_boundary},
};

#[path = "viewport.rs"]
mod viewport;
use viewport::{Camera, Pick, Scene};

const EXAMPLE: &str = include_str!("../../../../../examples/open-frame.obj");
const SIDES: [(&str, Axis); 6] = [
    ("Top (+Z)", Axis::Z),
    ("Bottom (−Z)", Axis::NegativeZ),
    ("Front (−Y)", Axis::NegativeY),
    ("Back (+Y)", Axis::Y),
    ("Right (+X)", Axis::X),
    ("Left (−X)", Axis::NegativeX),
];
const UNITS: [(&str, LengthUnit); 4] = [
    ("mm", LengthUnit::Millimetre),
    ("cm", LengthUnit::Centimetre),
    ("m", LengthUnit::Metre),
    ("in", LengthUnit::Inch),
];
const ACCENT: Color32 = Color32::from_rgb(100, 221, 199);
const MUTED: Color32 = Color32::from_rgb(156, 171, 183);

pub struct Studio {
    scene: Option<Scene>,
    camera: Camera,
    source_name: String,
    input: String,
    unit: usize,
    side: usize,
    anchors: Vec<usize>,
    selected: Option<usize>,
    skin: Option<Surface>,
    pattern: Option<DevelopedPanel>,
    show_skin: bool,
    select_edges: bool,
    output: String,
    stock_enabled: bool,
    width: String,
    length: String,
    geometry_error: String,
    message: Option<(bool, String)>,
}

impl Studio {
    pub fn new(context: &egui::Context) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(21, 27, 34);
        visuals.window_fill = visuals.panel_fill;
        visuals.widgets.active.bg_fill = Color32::from_rgb(31, 100, 94);
        visuals.selection.bg_fill = Color32::from_rgb(32, 90, 89);
        context.set_visuals(visuals);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".into(),
            egui::FontData::from_static(include_bytes!("../../../assets/InterVariable.ttf")).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".into());
        context.set_fonts(fonts);
        let mut studio = Self {
            scene: None,
            camera: Camera::default(),
            source_name: String::new(),
            input: String::new(),
            unit: 0,
            side: 0,
            anchors: Vec::new(),
            selected: None,
            skin: None,
            pattern: None,
            show_skin: true,
            select_edges: false,
            output: String::new(),
            stock_enabled: false,
            width: String::new(),
            length: String::new(),
            geometry_error: String::new(),
            message: None,
        };
        studio.open_example();
        studio
    }

    fn open_example(&mut self) {
        let name = Path::new("open-frame.obj");
        match parse_obj(EXAMPLE, name, LengthUnit::Millimetre) {
            Ok(meshes) => {
                let directory = std::env::current_exe()
                    .ok()
                    .and_then(|path| path.parent().map(Path::to_path_buf))
                    .unwrap_or_default();
                self.accept_model(
                    meshes,
                    "Open frame example".into(),
                    directory.join("frame-pattern.svg"),
                );
            }
            Err(error) => self.message = Some((false, error.to_string())),
        }
    }

    fn open_model(&mut self, path: PathBuf) {
        match import_obj(&path, UNITS[self.unit].1) {
            Ok(meshes) => {
                let name = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned();
                self.accept_model(meshes, name, path.with_extension("fabric.svg"));
            }
            Err(error) => self.message = Some((false, error.to_string())),
        }
    }

    fn accept_model(&mut self, meshes: Vec<ObjMesh>, name: String, output: PathBuf) {
        match Scene::from_meshes(&meshes) {
            Ok(scene) => {
                self.scene = Some(scene);
                self.source_name = name;
                self.output = output.to_string_lossy().into_owned();
                self.message = None;
                self.camera = Camera::default();
                self.anchors.clear();
                self.selected = None;
                self.suggest();
            }
            Err(error) => self.message = Some((false, error.into())),
        }
    }

    fn suggest(&mut self) {
        let Some(scene) = &self.scene else {
            return;
        };
        match suggest_boundary(&scene.points, SIDES[self.side].1) {
            Ok(indices) => {
                self.anchors = indices;
                self.selected = None;
                self.refresh();
            }
            Err(error) => self.message = Some((false, error.to_string())),
        }
    }

    fn refresh(&mut self) {
        self.geometry_error.clear();
        self.message = None;
        self.skin = None;
        self.pattern = None;
        let Some(scene) = &self.scene else {
            return;
        };
        if self.anchors.len() < 3 {
            return;
        }
        let points: Vec<_> = self.anchors.iter().map(|&id| scene.points[id]).collect();
        match build_skin(&points) {
            Ok(skin) => {
                match DevelopedPanel::from_surface(&skin) {
                    Ok(pattern) => self.pattern = Some(pattern),
                    Err(error) => self.geometry_error = error.to_string(),
                }
                self.skin = Some(skin);
            }
            Err(error) => self.geometry_error = error.to_string(),
        }
    }

    fn add_pick(&mut self, pick: Pick) {
        match pick {
            Pick::Point(id) => {
                if let Some(position) = self.anchors.iter().position(|&other| other == id) {
                    self.anchors.remove(position);
                    self.selected = None;
                } else {
                    let index = self
                        .selected
                        .map_or(self.anchors.len(), |selected| selected + 1);
                    self.anchors.insert(index, id);
                    self.selected = Some(index);
                }
            }
            Pick::Edge(ids) => {
                // Insert the edge endpoints after the selected boundary point.
                // If one endpoint is already selected, continue from that endpoint.
                if let Some((position, next)) =
                    ids.into_iter()
                        .zip(ids.into_iter().rev())
                        .find_map(|(current, next)| {
                            self.anchors
                                .iter()
                                .position(|&id| id == current)
                                .map(|index| (index, next))
                        })
                {
                    if self.anchors.contains(&next) {
                        self.selected = Some(position);
                        return;
                    }
                    self.anchors.insert(position + 1, next);
                    self.selected = Some(position + 1);
                } else {
                    let index = self
                        .selected
                        .map_or(self.anchors.len(), |selected| selected + 1);
                    self.anchors.splice(index..index, ids);
                    self.selected = Some(index + 1);
                }
            }
        }
        self.refresh();
    }

    fn export(&mut self) {
        let Some(pattern) = &self.pattern else {
            return;
        };
        let stock = if self.stock_enabled {
            match (
                self.width.trim().parse::<f64>(),
                self.length.trim().parse::<f64>(),
            ) {
                (Ok(width), Ok(length)) => Some([width, length]),
                _ => {
                    self.message = Some((
                        false,
                        "Enter the usable fabric width and length in millimetres.".into(),
                    ));
                    return;
                }
            }
        } else {
            None
        };
        let path = Path::new(self.output.trim().trim_matches('"'));
        self.message = Some(match save_panel_svg(pattern.panel(), path, stock) {
            Ok(fit) if fit.is_empty() => (true, format!("Saved {}", path.display())),
            Ok(fit) => (true, format!("Saved {}\n{fit}", path.display())),
            Err(error) => (false, error.to_string()),
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reference");
        ui.add(
            egui::TextEdit::singleline(&mut self.input)
                .hint_text("Drop an OBJ or paste its path")
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("units")
                .selected_text(UNITS[self.unit].0)
                .show_ui(ui, |ui| {
                    for (index, (name, _)) in UNITS.iter().enumerate() {
                        ui.selectable_value(&mut self.unit, index, *name);
                    }
                });
            if ui
                .add_enabled(!self.input.trim().is_empty(), egui::Button::new("Open OBJ"))
                .clicked()
            {
                self.open_model(PathBuf::from(self.input.trim().trim_matches('"')));
            }
            if ui.button("Frame example").clicked() {
                self.open_example();
            }
        });
        ui.add_space(12.);
        ui.separator();
        ui.add_space(8.);
        ui.heading("Fabric boundary");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("side")
                .selected_text(SIDES[self.side].0)
                .show_ui(ui, |ui| {
                    for (index, (name, _)) in SIDES.iter().enumerate() {
                        ui.selectable_value(&mut self.side, index, *name);
                    }
                });
            if ui
                .add_enabled(self.scene.is_some(), egui::Button::new("Suggest corners"))
                .clicked()
            {
                self.suggest();
            }
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.select_edges, false, "Pick vertex");
            ui.selectable_value(&mut self.select_edges, true, "Pick edge");
            ui.checkbox(&mut self.show_skin, "Show skin");
        });
        ui.label(
            RichText::new("Pick around the perimeter. Select a row to insert after it.")
                .color(MUTED)
                .small(),
        );
        egui::ScrollArea::vertical()
            .id_salt("anchors")
            .max_height(107.)
            .show(ui, |ui| {
                if let Some(scene) = &self.scene {
                    for (index, &anchor) in self.anchors.iter().enumerate() {
                        let [x, y, z] = scene.points[anchor];
                        let label = format!(
                            "{}   {}   ({x:.1}, {y:.1}, {z:.1})",
                            index + 1,
                            scene.labels[anchor]
                        );
                        if ui
                            .selectable_label(self.selected == Some(index), label)
                            .clicked()
                        {
                            self.selected = (self.selected != Some(index)).then_some(index);
                        }
                    }
                }
            });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.selected.is_some(), egui::Button::new("Remove"))
                .clicked()
                && let Some(index) = self.selected.take()
            {
                self.anchors.remove(index);
                self.refresh();
            }
            if ui
                .add_enabled(
                    !self.anchors.is_empty(),
                    egui::Button::new("Clear boundary"),
                )
                .clicked()
            {
                self.anchors.clear();
                self.selected = None;
                self.refresh();
            }
        });
        ui.add_space(12.);
        ui.separator();
        ui.add_space(8.);
        ui.heading("Pattern export");
        if let Some(pattern) = &self.pattern {
            let [w, h] = pattern.panel().size_mm();
            ui.label(
                RichText::new(format!("{w:.1} × {h:.1} mm"))
                    .color(ACCENT)
                    .strong(),
            );
        } else {
            ui.label(RichText::new("Select a developable fabric boundary.").color(MUTED));
        }
        if !self.geometry_error.is_empty() {
            ui.colored_label(Color32::from_rgb(253, 181, 124), &self.geometry_error);
        }
        ui.checkbox(&mut self.stock_enabled, "Check usable fabric size");
        if self.stock_enabled {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.width)
                        .hint_text("Width mm")
                        .desired_width(104.),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.length)
                        .hint_text("Length mm")
                        .desired_width(104.),
                );
            });
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.output)
                .hint_text("SVG output path")
                .desired_width(f32::INFINITY),
        );
        if ui
            .add_enabled(
                self.pattern.is_some(),
                egui::Button::new("Export full-scale SVG").fill(Color32::from_rgb(31, 102, 94)),
            )
            .clicked()
        {
            self.export();
        }
        if let Some((passed, message)) = &self.message {
            ui.colored_label(
                if *passed {
                    ACCENT
                } else {
                    Color32::from_rgb(253, 181, 124)
                },
                message,
            );
        }
    }
}

impl Studio {
    fn draw(&mut self, ctx: &egui::Context) {
        if let Some(path) = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        }) {
            self.input = path.to_string_lossy().into_owned();
            self.open_model(path);
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(5.);
            ui.horizontal(|ui| {
                ui.label(RichText::new("FabricCraft").size(22.).strong());
                ui.separator();
                ui.label(RichText::new(&self.source_name).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset view").clicked() {
                        self.camera = Camera::default();
                    }
                    if ui.button("Top view").clicked() {
                        self.camera.top();
                    }
                });
            });
            ui.add_space(5.);
        });
        egui::SidePanel::left("settings")
            .exact_width(305.)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings-scroll")
                    .show(ui, |ui| {
                        ui.add_space(7.);
                        self.sidebar(ui);
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Fabric skin");
                ui.label(RichText::new("Drag to orbit · Scroll to zoom").color(MUTED));
            });
            ui.label(
                RichText::new(
                    "Geometric preview. Stretch and collisions are outside this estimate.",
                )
                .small()
                .color(MUTED),
            );
            if let Some(scene) = &self.scene {
                let skin = if self.show_skin {
                    self.skin.as_ref()
                } else {
                    None
                };
                if let Some(pick) = self.camera.show(
                    ui,
                    scene,
                    &self.anchors,
                    self.selected,
                    skin,
                    self.select_edges,
                ) {
                    self.add_pick(pick);
                }
            }
        });
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.draw(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dimensions(studio: &Studio) {
        let [width, height] = studio.pattern.as_ref().unwrap().panel().size_mm();
        assert!((width - 600.).abs() < 1e-8);
        assert!((height - 400.).abs() < 1e-8);
    }

    #[test]
    fn previews_the_frame_and_exports_the_suggested_skin() {
        let context = egui::Context::default();
        let mut studio = Studio::new(&context);
        assert_eq!(studio.anchors.len(), 4);
        assert_eq!(studio.skin.as_ref().unwrap().record().faces.len(), 2);
        dimensions(&studio);
        let frame = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1120., 740.),
                )),
                ..Default::default()
            },
            |ctx| studio.draw(ctx),
        );
        assert!(!frame.shapes.is_empty());
        let dir = std::env::temp_dir().join(format!("fabriccraft-gui-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("frame.svg");
        studio.output = path.to_string_lossy().into_owned();
        studio.stock_enabled = true;
        studio.width = "610".into();
        studio.length = "420".into();
        studio.export();
        let result = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
        assert!(result.contains("Usable fabric 610 mm wide by 420 mm long"));
        assert!(result.contains("<polygon"));
    }

    #[test]
    fn can_replace_suggested_corners_with_reference_edges_and_vertices() {
        let mut studio = Studio::new(&egui::Context::default());
        let keys = [
            [0., 0., 45.],
            [600., 0., 45.],
            [600., 400., 45.],
            [0., 400., 45.],
        ];
        let scene = studio.scene.as_ref().unwrap();
        let [a, b, c, d] = keys.map(|point| scene.points.iter().position(|x| *x == point).unwrap());
        studio.anchors.clear();
        studio.selected = None;
        studio.add_pick(Pick::Edge([a, b]));
        studio.add_pick(Pick::Point(c));
        studio.add_pick(Pick::Point(d));
        assert_eq!(studio.anchors, [a, b, c, d]);
        dimensions(&studio);
        studio.add_pick(Pick::Point(c));
        assert_eq!(studio.anchors, [a, b, d]);
    }
}
