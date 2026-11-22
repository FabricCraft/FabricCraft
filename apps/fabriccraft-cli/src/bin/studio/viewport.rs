use std::collections::BTreeMap;

use eframe::egui::{self, Color32, Pos2, Shape, Stroke, Vec2};
use fabriccraft_cli::panel::ObjMesh;
use fabriccraft_core::{
    geometry::{Axis, Surface},
    planar::CutPattern,
    skin::Boundary,
};

const INK: Color32 = Color32::from_rgb(112, 237, 210);
const PICKED_EDGE: Color32 = Color32::from_rgb(255, 190, 111);
const BACKGROUND: Color32 = Color32::from_rgb(14, 19, 26);

pub struct Scene {
    pub points: Vec<[f64; 3]>,
    triangles: Vec<Triangle>,
    edges: Vec<([usize; 2], bool)>,
    center: [f64; 3],
    scale: f64,
}

struct Triangle {
    points: [usize; 3],
    creases: [bool; 3],
}

impl Scene {
    pub fn from_meshes(meshes: &[ObjMesh]) -> Result<Self, &'static str> {
        let mut points = Vec::new();
        let mut support_ids = BTreeMap::new();
        let mut triangles = Vec::new();
        let mut edges = BTreeMap::new();
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for mesh in meshes {
            let record = mesh.surface.record();
            let mut ids = BTreeMap::new();
            for v in &record.vertices_mm {
                if v.position.iter().any(|value| value.abs() > 1e9) {
                    return Err("The reference coordinates exceed this preview's range.");
                }
                // Joined OBJ parts can have distinct indices for the same
                // physical support. Use one pick target at that location.
                let key = v.position.map(|p| if p == 0. { 0 } else { p.to_bits() });
                let id = *support_ids.entry(key).or_insert_with(|| {
                    points.push(v.position);
                    points.len() - 1
                });
                ids.insert(v.id, id);
                for (axis, value) in v.position.into_iter().enumerate() {
                    low[axis] = low[axis].min(value);
                    high[axis] = high[axis].max(value);
                }
            }
            for f in &record.faces {
                let [a, b, c] = f.vertices.map(|id| ids[&id]);
                triangles.push([a, b, c]);
                let normal = unit_normal([a, b, c].map(|id| points[id]));
                for (i, j) in [(a, b), (b, c), (c, a)] {
                    let (first, count, sharp) = edges
                        .entry([i.min(j), i.max(j)])
                        .or_insert((normal, 0, false));
                    let cosine: f64 = first.iter().zip(normal).map(|(p, q)| p * q).sum();
                    *sharp |= cosine.abs() < 0.995;
                    *count += 1;
                }
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
        let edges: BTreeMap<_, _> = edges
            .into_iter()
            .map(|(edge, (_, count, sharp))| (edge, count != 2 || sharp))
            .collect();
        let triangles = triangles
            .into_iter()
            .map(|points| Triangle {
                points,
                creases: std::array::from_fn(|i| {
                    let [a, b] = [points[i], points[(i + 1) % 3]];
                    edges[&[a.min(b), a.max(b)]]
                }),
            })
            .collect();
        Ok(Self {
            points,
            triangles,
            edges: edges.into_iter().collect(),
            center,
            scale,
        })
    }
}

fn unit_normal([a, b, c]: [[f64; 3]; 3]) -> [f64; 3] {
    let u = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
    let v = std::array::from_fn::<_, 3, _>(|i| c[i] - a[i]);
    let [x, y, z] = std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        u[j] * v[k] - u[k] * v[j]
    });
    let length = x.hypot(y).hypot(z);
    [x / length, y / length, z / length]
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

struct ProjectedFace {
    corners: [Projected; 3],
    creases: [bool; 3],
}

impl Camera {
    pub fn look_at(&mut self, side: Axis) {
        use std::f64::consts::{FRAC_PI_2, PI};
        (self.yaw, self.pitch) = match side {
            Axis::Z => (0., FRAC_PI_2),
            Axis::NegativeZ => (0., -FRAC_PI_2),
            Axis::X => (-FRAC_PI_2, 0.),
            Axis::NegativeX => (FRAC_PI_2, 0.),
            Axis::Y => (PI, 0.),
            Axis::NegativeY => (0., 0.),
        };
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        scene: &Scene,
        boundary: &Boundary,
        skin: Option<&Surface>,
        select_edges: bool,
    ) -> Option<Pick> {
        let anchors = boundary.points();
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 8, BACKGROUND);
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = ui.input(|input| input.pointer.delta());
            self.yaw += f64::from(delta.x) * 0.009;
            let limit = std::f64::consts::FRAC_PI_2;
            self.pitch = (self.pitch + f64::from(delta.y) * 0.008).clamp(-limit, limit);
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
            .map(|triangle| ProjectedFace {
                corners: triangle.points.map(|id| projected[id]),
                creases: triangle.creases,
            })
            .collect();
        faces.sort_by(|a, b| average_depth(a.corners).total_cmp(&average_depth(b.corners)));
        for face in &faces {
            let p = face.corners.map(|corner| corner.pos);
            let n = cross_2d(p[1] - p[0], p[2] - p[0]);
            let color = if n < 0. {
                Color32::from_rgb(94, 91, 83)
            } else {
                Color32::from_rgb(117, 110, 94)
            };
            painter.add(Shape::convex_polygon(p.to_vec(), color, Stroke::NONE));
            for (i, crease) in face.creases.into_iter().enumerate() {
                if crease {
                    painter.line_segment(
                        [p[i], p[(i + 1) % 3]],
                        Stroke::new(1_f32, Color32::from_rgb(177, 166, 141)),
                    );
                }
            }
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
                    anchor_positions,
                    Stroke::new(2_f32, INK),
                ));
            } else {
                painter.add(Shape::line(anchor_positions, Stroke::new(2_f32, INK)));
            }
        }
        for &edge in boundary.edges() {
            painter.line_segment(
                edge.map(|id| projected[id].pos),
                Stroke::new(4_f32, PICKED_EDGE),
            );
        }
        for &id in anchors {
            let point = projected[id].pos;
            painter.circle_filled(point, 4.5, INK);
            painter.circle_stroke(point, 4.5, Stroke::new(1.5_f32, BACKGROUND));
        }

        let point = response.hover_pos().filter(|_| !response.dragged())?;
        let depth = front_depth(point, &faces);
        let candidate = if select_edges {
            best_edge(point, depth, &projected, &scene.edges)
        } else {
            best_vertex(point, depth, &projected)
        };
        if let Some(pick) = candidate {
            match pick {
                Pick::Point(id) => {
                    painter.circle_stroke(
                        projected[id].pos,
                        8.,
                        Stroke::new(2_f32, Color32::WHITE),
                    );
                }
                Pick::Edge([a, b]) => {
                    painter.line_segment(
                        [projected[a].pos, projected[b].pos],
                        Stroke::new(3_f32, Color32::WHITE),
                    );
                }
            }
            if response.clicked() {
                return Some(pick);
            }
        }
        None
    }
}

pub fn show_pattern(ui: &mut egui::Ui, pattern: Option<&CutPattern>) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 8, BACKGROUND);
    let Some(pattern) = pattern else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No cut pattern available",
            egui::FontId::proportional(14.),
            Color32::LIGHT_GRAY,
        );
        return;
    };
    let [width, height] = pattern.size_mm();
    let scale =
        (f64::from(rect.width() * 0.86) / width).min(f64::from(rect.height() * 0.82) / height);
    let project = |&[x, y]: &[f64; 2]| {
        rect.center()
            + Vec2::new(
                ((x - width / 2.) * scale) as f32,
                ((height / 2. - y) * scale) as f32,
            )
    };
    painter.add(Shape::closed_line(
        pattern.cut_line_mm().iter().map(project).collect(),
        Stroke::new(2_f32, INK),
    ));
    if pattern.allowance_mm() > 0. {
        let mut sewing: Vec<_> = pattern.sewing_line_mm().iter().map(project).collect();
        sewing.push(sewing[0]);
        painter.extend(Shape::dashed_line(
            &sewing,
            Stroke::new(1.5_f32, PICKED_EDGE),
            5.,
            4.,
        ));
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

fn front_depth(pos: Pos2, faces: &[ProjectedFace]) -> f32 {
    faces
        .iter()
        .filter_map(|face| {
            let [a, b, c] = face.corners;
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

fn best_edge(
    pos: Pos2,
    depth: f32,
    vertices: &[Projected],
    edges: &[([usize; 2], bool)],
) -> Option<Pick> {
    edges
        .iter()
        .filter_map(|&(edge, feature)| {
            let [a, b] = edge.map(|id| vertices[id]);
            let line = b.pos - a.pos;
            let length = line.length_sq();
            if length < 1. {
                return None;
            }
            let fraction = ((pos - a.pos).dot(line) / length).clamp(0., 1.);
            let distance = pos.distance(a.pos + fraction * line);
            let z = a.depth + fraction * (b.depth - a.depth);
            let score = distance + if feature { 0. } else { 2. };
            (distance < 9. && z + 0.025 >= depth).then_some((edge, score, z))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.total_cmp(&a.2)))
        .map(|v| Pick::Edge(v.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabriccraft_cli::panel::parse_obj;
    use fabriccraft_core::geometry::LengthUnit;
    use std::path::Path;

    #[test]
    fn hides_coplanar_diagonals_and_prefers_a_visible_reference_edge() {
        let meshes = parse_obj(
            "v 0 0 0\nv 200 0 0\nv 200 100 0\nv 0 100 0\nf 1 2 3\nf 1 3 4\n",
            Path::new("rectangle.obj"),
            LengthUnit::Millimetre,
        )
        .unwrap();
        let scene = Scene::from_meshes(&meshes).unwrap();
        assert_eq!(scene.edges.len(), 5);
        assert_eq!(scene.edges.iter().filter(|(_, crease)| *crease).count(), 4);
        assert!(scene.edges.contains(&([0, 2], false)));
        assert_eq!(scene.triangles[0].creases, [true, true, false]);
        let points: Vec<_> = scene
            .points
            .iter()
            .map(|&[x, y, z]| Projected {
                pos: egui::pos2(x as f32, y as f32),
                depth: z as f32,
            })
            .collect();
        let target = egui::pos2(8., 2.);
        assert!(matches!(
            best_edge(target, 0., &points, &scene.edges),
            Some(Pick::Edge([0, 1]))
        ));
        assert!(best_edge(target, 1., &points, &scene.edges).is_none());
    }
}
