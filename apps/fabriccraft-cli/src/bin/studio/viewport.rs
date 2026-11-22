use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use fabriccraft_cli::panel::ObjMesh;
use fabriccraft_core::geometry::Surface;

const INK: Color32 = Color32::from_rgb(112, 237, 210);
const BACKGROUND: Color32 = Color32::from_rgb(14, 19, 26);

pub struct Scene {
    pub points: Vec<[f64; 3]>,
    pub labels: Vec<String>,
    triangles: Vec<[usize; 3]>,
    edges: Vec<[usize; 2]>,
    center: [f64; 3],
    scale: f64,
}

impl Scene {
    pub fn from_meshes(meshes: &[ObjMesh]) -> Result<Self, &'static str> {
        let mut points = Vec::new();
        let mut labels = Vec::new();
        let mut triangles = Vec::new();
        let mut edges = BTreeSet::new();
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for (body, mesh) in meshes.iter().enumerate() {
            let record = mesh.surface.record();
            let mut ids = BTreeMap::new();
            for v in &record.vertices_mm {
                if v.position.iter().any(|value| value.abs() > 1e9) {
                    return Err("The reference coordinates exceed this preview's range.");
                }
                ids.insert(v.id, points.len());
                points.push(v.position);
                labels.push(format!("{}:{}", body + 1, v.id.0 + 1));
                for (axis, value) in v.position.into_iter().enumerate() {
                    low[axis] = low[axis].min(value);
                    high[axis] = high[axis].max(value);
                }
            }
            for f in &record.faces {
                let [a, b, c] = f.vertices.map(|id| ids[&id]);
                triangles.push([a, b, c]);
                edges.extend([(a, b), (b, c), (c, a)].map(|(i, j)| [i.min(j), i.max(j)]));
            }
        }
        if triangles.is_empty() {
            return Err("The OBJ contains no triangles.");
        }
        let center = std::array::from_fn(|i| low[i].midpoint(high[i]));
        let scale = (0..3).map(|i| high[i] - low[i]).fold(0., f64::max);
        if scale == 0. {
            return Err("The reference has collapsed bounds.");
        }
        Ok(Self {
            points,
            labels,
            triangles,
            edges: edges.into_iter().collect(),
            center,
            scale,
        })
    }
}

#[derive(Clone, Copy)]
pub enum Pick {
    Point(usize),
    Edge([usize; 2]),
}

pub struct Camera {
    yaw: f64,
    pitch: f64,
    zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.52,
            pitch: 0.61,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Projected {
    pos: Pos2,
    depth: f32,
}

impl Camera {
    pub fn top(&mut self) {
        self.pitch = std::f64::consts::FRAC_PI_2;
        self.yaw = 0.;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        scene: &Scene,
        anchors: &[usize],
        selected: Option<usize>,
        skin: Option<&Surface>,
        select_edges: bool,
    ) -> Option<Pick> {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 8, BACKGROUND);
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = ui.input(|input| input.pointer.delta());
            self.yaw += f64::from(delta.x) * 0.009;
            self.pitch =
                (self.pitch + f64::from(delta.y) * 0.008).clamp(-1.57, std::f64::consts::FRAC_PI_2);
        }
        if response.hovered() {
            let wheel = ui.input(|input| input.smooth_scroll_delta.y);
            self.zoom = (self.zoom * (wheel * 0.002).exp()).clamp(0.25, 7.);
        }

        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let size = rect.width().min(rect.height()) * 0.73 * self.zoom;
        let project = |p: [f64; 3]| {
            let [x, y, z] = std::array::from_fn(|i| (p[i] - scene.center[i]) / scene.scale);
            let horizontal = cos_yaw * x - sin_yaw * y;
            let across = sin_yaw * x + cos_yaw * y;
            let vertical = cos_pitch * z + sin_pitch * across;
            let depth = sin_pitch * z - cos_pitch * across;
            Projected {
                pos: rect.center() + Vec2::new(horizontal as f32, -vertical as f32) * size,
                depth: depth as f32,
            }
        };
        let projected: Vec<_> = scene.points.iter().copied().map(project).collect();
        paint_grid(&painter, scene, &project);
        let mut faces: Vec<_> = scene
            .triangles
            .iter()
            .map(|triangle| triangle.map(|id| projected[id]))
            .collect();
        faces.sort_by(|a, b| average_depth(*a).total_cmp(&average_depth(*b)));
        for face in &faces {
            let p = face.map(|corner| corner.pos);
            let n = cross_2d(p[1] - p[0], p[2] - p[0]);
            let color = if n < 0. {
                Color32::from_rgb(94, 91, 83)
            } else {
                Color32::from_rgb(117, 110, 94)
            };
            painter.add(Shape::convex_polygon(
                p.to_vec(),
                color,
                Stroke::new(0.75_f32, Color32::from_rgb(159, 149, 125)),
            ));
        }

        if let Some(skin) = skin {
            let mut preview: Vec<_> = skin
                .record()
                .faces
                .iter()
                .map(|face| face.vertices.map(|id| projected[anchors[id.0 as usize]]))
                .collect();
            preview.sort_by(|a, b| average_depth(*a).total_cmp(&average_depth(*b)));
            for face in preview {
                painter.add(Shape::convex_polygon(
                    face.map(|corner| corner.pos).to_vec(),
                    Color32::from_rgba_unmultiplied(57, 199, 178, 108),
                    Stroke::NONE,
                ));
            }
        }

        let anchor_positions: Vec<_> = anchors.iter().map(|&id| projected[id].pos).collect();
        if anchor_positions.len() > 1 {
            if anchor_positions.len() > 2 {
                painter.add(Shape::closed_line(
                    anchor_positions.clone(),
                    Stroke::new(2_f32, INK),
                ));
            } else {
                painter.add(Shape::line(
                    anchor_positions.clone(),
                    Stroke::new(2_f32, INK),
                ));
            }
        }
        for (index, point) in anchor_positions.iter().enumerate() {
            let active = selected == Some(index);
            painter.circle_filled(*point, if active { 7. } else { 5. }, INK);
            painter.circle_stroke(
                *point,
                if active { 7. } else { 5. },
                Stroke::new(1.5_f32, BACKGROUND),
            );
            painter.text(
                *point + Vec2::new(8., -7.),
                egui::Align2::LEFT_BOTTOM,
                (index + 1).to_string(),
                egui::FontId::monospace(13.),
                INK,
            );
        }

        let point = response.hover_pos().filter(|_| !response.dragged())?;
        let depth = front_depth(point, &faces);
        let candidate = if select_edges {
            best_edge(point, depth, &projected, &scene.edges)
        } else {
            best_vertex(point, depth, &projected)
        };
        if let Some(pick) = candidate {
            let label = match pick {
                Pick::Point(id) => {
                    painter.circle_stroke(
                        projected[id].pos,
                        8.,
                        Stroke::new(2_f32, Color32::WHITE),
                    );
                    scene.labels[id].clone()
                }
                Pick::Edge([a, b]) => {
                    painter.line_segment(
                        [projected[a].pos, projected[b].pos],
                        Stroke::new(3_f32, Color32::WHITE),
                    );
                    format!("{} – {}", scene.labels[a], scene.labels[b])
                }
            };
            painter.text(
                rect.left_top() + Vec2::new(13., 11.),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::monospace(13.),
                INK,
            );
            if response.clicked() {
                return Some(pick);
            }
        }
        None
    }
}

fn average_depth(f: [Projected; 3]) -> f32 {
    (f[0].depth + f[1].depth + f[2].depth) / 3.
}

fn paint_grid(painter: &egui::Painter, scene: &Scene, project: &impl Fn([f64; 3]) -> Projected) {
    let [x, y, z] = scene.center;
    let gap = scene.scale * 0.10;
    let floor = z - scene.scale * 0.23;
    let stroke = Stroke::new(0.5_f32, Color32::from_rgb(35, 45, 54));
    for i in -7..=7 {
        let offset = f64::from(i) * gap;
        for points in [
            [
                [x + offset, y - 7. * gap, floor],
                [x + offset, y + 7. * gap, floor],
            ],
            [
                [x - 7. * gap, y + offset, floor],
                [x + 7. * gap, y + offset, floor],
            ],
        ] {
            painter.line_segment(points.map(|p| project(p).pos), stroke);
        }
    }
}

fn front_depth(pos: Pos2, faces: &[[Projected; 3]]) -> f32 {
    faces
        .iter()
        .filter_map(|&[a, b, c]| {
            let area = cross_2d(b.pos - a.pos, c.pos - a.pos);
            if area.abs() < 0.01 {
                return None;
            }
            let u = cross_2d(b.pos - pos, c.pos - pos) / area;
            let v = cross_2d(c.pos - pos, a.pos - pos) / area;
            let w = 1. - u - v;
            (u >= 0. && v >= 0. && w >= 0.).then_some(u * a.depth + v * b.depth + w * c.depth)
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

fn cross_2d(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn best_vertex(pos: Pos2, depth: f32, vertices: &[Projected]) -> Option<Pick> {
    vertices
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            let distance = pos.distance_sq(point.pos);
            (distance < 81. && point.depth + 0.025 >= depth).then_some((
                index,
                distance,
                point.depth,
            ))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.total_cmp(&a.2)))
        .map(|v| Pick::Point(v.0))
}

fn best_edge(pos: Pos2, depth: f32, vertices: &[Projected], edges: &[[usize; 2]]) -> Option<Pick> {
    edges
        .iter()
        .filter_map(|&edge| {
            let [a, b] = edge.map(|id| vertices[id]);
            let line = b.pos - a.pos;
            let length = line.length_sq();
            if length < 1. {
                return None;
            }
            let fraction = ((pos - a.pos).dot(line) / length).clamp(0., 1.);
            let distance = pos.distance_sq(a.pos + fraction * line);
            let z = a.depth + fraction * (b.depth - a.depth);
            (distance < 64. && z + 0.025 >= depth).then_some((edge, distance, z))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.total_cmp(&a.2)))
        .map(|v| Pick::Edge(v.0))
}
