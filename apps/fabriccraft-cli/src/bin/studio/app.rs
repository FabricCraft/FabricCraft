use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText};
use fabriccraft_cli::panel::{ObjMesh, import_obj, parse_obj, save_cut_pattern_svg};
use fabriccraft_core::{
    develop::DevelopedPanel,
    geometry::{Axis, LengthUnit, Surface},
    planar::CutPattern,
    skin::Boundary,
};

#[path = "measurement.rs"]
mod measurement;
#[path = "viewport.rs"]
mod viewport;
use measurement::Measurement;
use viewport::{Camera, Pick, Scene, show_pattern};

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
const WARNING: Color32 = Color32::from_rgb(253, 181, 124);

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Model,
    Pattern,
    Both,
}

pub struct Studio {
    scene: Option<Scene>,
    camera: Camera,
    source_name: String,
    input: String,
    unit: usize,
    side: usize,
    boundary: Boundary,
    previous_boundary: Option<Boundary>,
    skin: Option<Surface>,
    pattern: Option<DevelopedPanel>,
    cut: Option<CutPattern>,
    allowance: Measurement,
    allowance_error: Option<String>,
    view: View,
    show_skin: bool,
    select_edges: bool,
    output: String,
    stock_enabled: bool,
    width: Measurement,
    length: Measurement,
    geometry_error: String,
    pick_error: Option<String>,
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
            boundary: Boundary::new(SIDES[0].1),
            previous_boundary: None,
            skin: None,
            pattern: None,
            cut: None,
            allowance: Measurement {
                text: "0".into(),
                ..Default::default()
            },
            allowance_error: None,
            view: View::Model,
            show_skin: true,
            select_edges: false,
            output: String::new(),
            stock_enabled: false,
            width: Measurement::default(),
            length: Measurement::default(),
            geometry_error: String::new(),
            pick_error: None,
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
                self.boundary = Boundary::new(SIDES[self.side].1);
                self.view = View::Model;
                self.refresh();
                self.suggest();
                self.previous_boundary = None;
            }
            Err(error) => self.message = Some((false, error.into())),
        }
    }

    fn suggest(&mut self) {
        let Some(scene) = &self.scene else {
            return;
        };
        match Boundary::suggest(&scene.points, SIDES[self.side].1) {
            Ok(boundary) => self.set_boundary(boundary),
            Err(error) => self.message = Some((false, error.to_string())),
        }
    }

    fn refresh(&mut self) {
        self.geometry_error.clear();
        self.pick_error = None;
        self.message = None;
        self.skin = None;
        self.pattern = None;
        if let Some(scene) = &self.scene
            && self.boundary.points().len() >= 3
        {
            match self.boundary.skin(&scene.points) {
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
        self.refresh_cut();
    }

    fn refresh_cut(&mut self) {
        self.cut = None;
        self.allowance_error = None;
        self.message = None;
        let Some(pattern) = &self.pattern else {
            return;
        };
        let Some(mm) = self.allowance.millimetres() else {
            self.allowance_error = Some("Enter a seam allowance of zero or more.".into());
            return;
        };
        match CutPattern::new(pattern.panel(), mm) {
            Ok(cut) => self.cut = Some(cut),
            Err(error) => self.allowance_error = Some(error.to_string()),
        }
    }

    fn set_boundary(&mut self, boundary: Boundary) {
        self.previous_boundary = Some(std::mem::replace(&mut self.boundary, boundary));
        self.refresh();
    }

    fn add_pick(&mut self, pick: Pick) {
        let Some(scene) = &self.scene else {
            return;
        };
        let result = match pick {
            Pick::Point(id) => self.boundary.toggle_point(&scene.points, id),
            Pick::Edge(edge) => self.boundary.toggle_edge(&scene.points, edge),
        };
        match result {
            Ok(boundary) => self.set_boundary(boundary),
            Err(error) => self.pick_error = Some(error.to_string()),
        }
    }

    fn export(&mut self) {
        let Some(pattern) = &self.cut else {
            return;
        };
        let stock = if self.stock_enabled {
            match (self.width.millimetres(), self.length.millimetres()) {
                (Some(width), Some(length)) if width > 0. && length > 0. => Some([width, length]),
                _ => {
                    self.message = Some((
                        false,
                        "Enter a positive usable fabric width and length.".into(),
                    ));
                    return;
                }
            }
        } else {
            None
        };
        let path = Path::new(self.output.trim().trim_matches('"'));
        self.message = Some(match save_cut_pattern_svg(pattern, path, stock) {
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
                })
                .response
                .on_hover_text("Coverage side for Suggest corners and Clear boundary");
            if ui
                .add_enabled(self.scene.is_some(), egui::Button::new("Suggest corners"))
                .clicked()
            {
                self.suggest();
                self.camera.look_at(self.boundary.side());
            }
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.select_edges, false, "Pick vertex");
            ui.selectable_value(&mut self.select_edges, true, "Pick edge");
        });
        ui.label(
            RichText::new(if self.select_edges {
                "Click a model edge to keep it on the boundary. Click it again to release it."
            } else {
                "Click supports in any order. Click a selected point again to remove it."
            })
            .color(MUTED)
            .small(),
        );
        ui.checkbox(&mut self.show_skin, "Show fabric skin");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.previous_boundary.is_some(),
                    egui::Button::new("Undo edit"),
                )
                .clicked()
                && let Some(boundary) = self.previous_boundary.take()
            {
                self.boundary = boundary;
                self.side = SIDES
                    .iter()
                    .position(|&(_, axis)| axis == self.boundary.side())
                    .unwrap();
                self.refresh();
            }
            if ui
                .add_enabled(
                    !self.boundary.points().is_empty(),
                    egui::Button::new("Clear boundary"),
                )
                .clicked()
            {
                self.set_boundary(Boundary::new(SIDES[self.side].1));
                self.camera.look_at(self.boundary.side());
            }
        });
        if let Some(message) = &self.pick_error {
            ui.colored_label(WARNING, message);
        }
        ui.add_space(12.);
        ui.separator();
        ui.add_space(8.);
        ui.heading("Cut pattern");
        if self.allowance.ui(ui, "Seam allowance") {
            self.refresh_cut();
        }
        if let Some(cut) = &self.cut {
            let [w, h] = cut.size_mm();
            ui.label(
                RichText::new(format!("{w:.1} × {h:.1} mm cut size"))
                    .color(ACCENT)
                    .strong(),
            );
            ui.label(RichText::new(format!("{:.2} × {:.2} in", w / 25.4, h / 25.4)).color(MUTED));
        } else if self.pattern.is_none() {
            ui.label(RichText::new("Select a developable fabric boundary.").color(MUTED));
        }
        if let Some(error) = &self.allowance_error {
            ui.colored_label(WARNING, error);
        }
        if !self.geometry_error.is_empty() {
            ui.colored_label(WARNING, &self.geometry_error);
        }
        ui.checkbox(&mut self.stock_enabled, "Check usable fabric size");
        if self.stock_enabled {
            self.width.ui(ui, "Fabric width");
            self.length.ui(ui, "Fabric length");
        }
        ui.add(
            egui::TextEdit::singleline(&mut self.output)
                .hint_text("SVG output path")
                .desired_width(f32::INFINITY),
        );
        if ui
            .add_enabled(
                self.cut.is_some(),
                egui::Button::new("Export full-scale SVG").fill(Color32::from_rgb(31, 102, 94)),
            )
            .clicked()
        {
            self.export();
        }
        if let Some((passed, message)) = &self.message {
            ui.colored_label(if *passed { ACCENT } else { WARNING }, message);
        }
    }
}

impl Studio {
    fn draw_model(&mut self, ui: &mut egui::Ui) {
        let Some(scene) = &self.scene else {
            return;
        };
        let skin = self.skin.as_ref().filter(|_| self.show_skin);
        if let Some(pick) = self
            .camera
            .show(ui, scene, &self.boundary, skin, self.select_edges)
        {
            self.add_pick(pick);
            ui.ctx().request_repaint();
        }
    }

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
                    if ui.button("Coverage view").clicked() {
                        self.camera.look_at(self.boundary.side());
                        if self.view == View::Pattern {
                            self.view = View::Model;
                        }
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
                ui.selectable_value(&mut self.view, View::Model, "Model");
                ui.selectable_value(&mut self.view, View::Pattern, "Pattern");
                ui.selectable_value(&mut self.view, View::Both, "Side by side");
            });
            if self.view != View::Pattern {
                ui.label(
                    RichText::new("Drag to orbit · Scroll to zoom · Geometric skin estimate")
                        .small()
                        .color(MUTED),
                );
            }
            if self.view != View::Model {
                ui.label(
                    RichText::new(match &self.cut {
                        Some(cut) if cut.allowance_mm() > 0. => {
                            "Cut line (solid) · Sewing line (dashed)"
                        }
                        _ => "Cut line",
                    })
                    .small()
                    .color(MUTED),
                );
            }
            match self.view {
                View::Model => self.draw_model(ui),
                View::Pattern => show_pattern(ui, self.cut.as_ref()),
                View::Both => ui.columns(2, |columns| {
                    self.draw_model(&mut columns[0]);
                    show_pattern(&mut columns[1], self.cut.as_ref());
                }),
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
        assert_eq!(studio.boundary.points().len(), 4);
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
        studio.width.text = "610".into();
        studio.length.text = "420".into();
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
        studio.set_boundary(Boundary::new(Axis::Z));
        studio.add_pick(Pick::Edge([a, b]));
        studio.add_pick(Pick::Point(c));
        studio.add_pick(Pick::Point(d));
        assert_eq!(studio.boundary.points().len(), 4);
        assert_eq!(studio.boundary.edges().len(), 1);
        dimensions(&studio);
        studio.add_pick(Pick::Point(c));
        assert_eq!(studio.boundary.points().len(), 3);
    }

    #[test]
    fn previews_and_exports_the_edited_cut_line_with_a_mixed_unit_stock_check() {
        let context = egui::Context::default();
        let mut studio = Studio::new(&context);
        let dir =
            std::env::temp_dir().join(format!("fabriccraft-edited-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let original = dir.join("original.svg");
        studio.output = original.to_string_lossy().into_owned();
        studio.export();

        let scene = studio.scene.as_ref().unwrap();
        let [a, b, c, d] = [
            [0., 0., 45.],
            [600., 0., 45.],
            [0., 40., 45.],
            [600., 40., 45.],
        ]
        .map(|point| scene.points.iter().position(|x| *x == point).unwrap());
        studio.add_pick(Pick::Point(b));
        studio.add_pick(Pick::Point(a));
        studio.add_pick(Pick::Edge([c, d]));
        let [width, height] = studio.cut.as_ref().unwrap().size_mm();
        assert!((width - 600.).abs() < 1e-8 && (height - 360.).abs() < 1e-8);

        studio.allowance = Measurement {
            text: "1/2".into(),
            unit: 3,
        };
        studio.refresh_cut();
        let [width, height] = studio.cut.as_ref().unwrap().size_mm();
        assert!((width - 625.4).abs() < 1e-8 && (height - 385.4).abs() < 1e-8);
        studio.view = View::Both;
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

        studio.stock_enabled = true;
        studio.width.text = "610".into();
        studio.length = Measurement {
            text: "16".into(),
            unit: 3,
        };
        let edited = dir.join("edited.svg");
        studio.output = edited.to_string_lossy().into_owned();
        studio.export();
        assert!(matches!(studio.message, Some((false, _))));
        assert!(!edited.exists());
        studio.width.text = "630".into();
        studio.export();

        let svg = std::fs::read_to_string(&edited).unwrap();
        assert_ne!(svg, std::fs::read_to_string(&original).unwrap());
        assert!(svg.contains("Seam allowance 12.7 mm"));
        assert!(svg.contains("id=\"sewing-line\""));
        assert!(svg.contains("Usable fabric 630 mm wide by 406.4 mm long"));
        std::fs::remove_file(original).unwrap();
        std::fs::remove_file(edited).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
