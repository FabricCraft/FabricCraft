use crate::{
    geometry::{Axis, Surface},
    planar::{MAX_BOUNDARY_EDGES, intersects, overlaps_at_join, validate_outline},
};

use super::{SkinError, check_coordinates, project, side_axis, suggest_boundary, triangulate_skin};

/// An editable fabric perimeter on one side of a reference model.
/// Edits return a new boundary so a rejected pick leaves the old one usable.
#[derive(Clone, Debug)]
pub struct Boundary {
    side: Axis,
    points: Vec<usize>,
    edges: Vec<[usize; 2]>,
}

impl Boundary {
    pub fn new(side: Axis) -> Self {
        Self {
            side,
            points: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn suggest(reference: &[[f64; 3]], side: Axis) -> Result<Self, SkinError> {
        Ok(Self {
            points: suggest_boundary(reference, side)?,
            ..Self::new(side)
        })
    }

    pub fn side(&self) -> Axis {
        self.side
    }

    pub fn points(&self) -> &[usize] {
        &self.points
    }

    /// Chosen reference edges, stored with ascending endpoint IDs.
    pub fn edges(&self) -> &[[usize; 2]] {
        &self.edges
    }

    pub fn skin(&self, reference: &[[f64; 3]]) -> Result<Surface, SkinError> {
        let anchors = self.positions(reference, &[])?;
        triangulate_skin(&anchors, Some(side_axis(self.side).0))
    }

    pub fn toggle_point(&self, reference: &[[f64; 3]], id: usize) -> Result<Self, SkinError> {
        let positions = self.positions(reference, &[id])?;
        let mut projected = self.project(&positions);
        if let Some(slot) = self.points.iter().position(|&point| point == id) {
            projected.pop();
            projected.remove(slot);
            validate(&projected)?;
            let mut next = self.clone();
            next.points.remove(slot);
            next.edges.retain(|edge| !edge.contains(&id));
            return Ok(next);
        }
        self.insert(&projected, &[&[id]], None)
    }

    pub fn toggle_edge(
        &self,
        reference: &[[f64; 3]],
        mut edge: [usize; 2],
    ) -> Result<Self, SkinError> {
        edge.sort_unstable();
        let [a, b] = edge;
        if a == b {
            return Err(SkinError::BoundaryEdit);
        }
        let len = self.points.len();
        let positions = self.positions(reference, &edge)?;
        let projected = self.project(&positions);
        if projected[len] == projected[len + 1] {
            return Err(SkinError::BoundaryEdit);
        }

        if let Ok(index) = self.edges.binary_search(&edge) {
            let mut next = self.clone();
            next.edges.remove(index);
            return Ok(next);
        }

        let [first, second] = edge.map(|id| self.points.iter().position(|&point| point == id));
        let mut next = match (first, second) {
            (None, None) => {
                let reversed = [b, a];
                self.insert(&projected, &[&edge, &reversed], None)?
            }
            (Some(slot), None) => {
                let mut projected = projected;
                projected.remove(len);
                self.insert(&projected, &[&[b]], Some(slot))?
            }
            (None, Some(slot)) => self.insert(&projected[..len + 1], &[&[a]], Some(slot))?,
            (Some(i), Some(j)) if i.abs_diff(j) == 1 || i.abs_diff(j) == len - 1 => self.clone(),
            (Some(i), Some(j)) => [(i, j), (j, i)]
                .into_iter()
                .filter(|&(moving, _)| !self.edges.iter().any(|e| e.contains(&self.points[moving])))
                .filter_map(|(moving, staying)| {
                    let mut shortened = self.clone();
                    let id = shortened.points.remove(moving);
                    let mut coords = projected[..len].to_vec();
                    let point = coords.remove(moving);
                    coords.push(point);
                    shortened
                        .insert(
                            &coords,
                            &[&[id]],
                            Some(staying - usize::from(staying > moving)),
                        )
                        .ok()
                })
                .min_by(|a, b| {
                    let perimeter = |boundary: &Self| {
                        let (axis, _) = side_axis(self.side);
                        loop_edges(&boundary.points)
                            .map(|(i, j)| {
                                distance(project(reference[i], axis), project(reference[j], axis))
                            })
                            .sum::<f64>()
                    };
                    perimeter(a).total_cmp(&perimeter(b))
                })
                .ok_or(SkinError::BoundaryEdit)?,
        };
        let index = next.edges.binary_search(&edge).unwrap_err();
        next.edges.insert(index, edge);
        Ok(next)
    }

    // The projected positions follow the current outline, then the first
    // insertion sequence. A second sequence reverses the same two points.
    fn insert(
        &self,
        projected: &[[f64; 2]],
        sequences: &[&[usize]],
        required_neighbor: Option<usize>,
    ) -> Result<Self, SkinError> {
        let n = self.points.len();
        if projected.len() > MAX_BOUNDARY_EDGES {
            return Err(SkinError::TooManyAnchors);
        }
        let mut next = self.clone();
        if n <= 1 {
            validate(projected)?;
            next.points.extend_from_slice(sequences[0]);
            return Ok(next);
        }
        let mut gaps = Vec::new();
        for (i, (a, b)) in loop_edges(&self.points).enumerate() {
            let j = (i + 1) % n;
            let mut edge = [a, b];
            edge.sort_unstable();
            if required_neighbor.is_some_and(|neighbor| neighbor != i && neighbor != j)
                || (n > 2 && self.edges.binary_search(&edge).is_ok())
            {
                continue;
            }
            for (direction, added) in sequences.iter().enumerate() {
                let first = n + direction;
                let last = n + added.len() - 1 - direction;
                let path = [i, first, last, j];
                let segments = [(i, first), (first, last), (last, j)];
                let cost = segments
                    .into_iter()
                    .map(|(k, l)| distance(projected[k], projected[l]))
                    .sum::<f64>()
                    - distance(projected[i], projected[j]);
                gaps.push((cost, direction, i, path));
            }
        }
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, direction, i, path) in gaps {
            if !insertion_is_simple(projected, n, path) {
                continue;
            }
            let mut outline = projected[..n].to_vec();
            let [_, first, last, _] = path;
            outline.insert(i + 1, projected[first]);
            if first != last {
                outline.insert(i + 2, projected[last]);
            }
            // Also check unchanged segments. Moving an existing point may
            // have added a shortcut where it used to be.
            if validate(&outline).is_ok() {
                next.points
                    .splice(i + 1..i + 1, sequences[direction].iter().copied());
                return Ok(next);
            }
        }
        Err(SkinError::BoundaryEdit)
    }

    fn positions(
        &self,
        reference: &[[f64; 3]],
        added: &[usize],
    ) -> Result<Vec<[f64; 3]>, SkinError> {
        let positions = self
            .points
            .iter()
            .chain(added)
            .map(|&id| reference.get(id).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(SkinError::BoundaryEdit)?;
        check_coordinates(&positions)?;
        Ok(positions)
    }

    fn project(&self, positions: &[[f64; 3]]) -> Vec<[f64; 2]> {
        positions
            .iter()
            .map(|&p| project(p, side_axis(self.side).0))
            .collect()
    }
}

fn validate(outline: &[[f64; 2]]) -> Result<(), SkinError> {
    if let [a, b] = outline {
        if a == b {
            return Err(SkinError::BoundaryEdit);
        }
    } else if outline.len() >= 3 {
        validate_outline(outline).map_err(|_| SkinError::BoundaryEdit)?;
    }
    Ok(())
}

fn loop_edges(ids: &[usize]) -> impl Iterator<Item = (usize, usize)> {
    ids.iter().copied().zip(ids.iter().copied().cycle().skip(1))
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

// Reject crossings in the new edges before running the full outline check.
// Only two or three edges change, regardless of the size of the outline.
fn insertion_is_simple(projected: &[[f64; 2]], n: usize, [a, p, q, b]: [usize; 4]) -> bool {
    let segments = [(a, p), (p, q), (q, b)];
    for (i, &(u, v)) in segments.iter().enumerate().filter(|(_, (u, v))| u != v) {
        let [start, end] = [projected[u], projected[v]];
        if start == end {
            return false;
        }
        let conflicts = |(x, y): (usize, usize)| {
            if x == y {
                return false;
            }
            let [from, to] = [projected[x], projected[y]];
            if v == x {
                overlaps_at_join(start, end, to)
            } else if u == y {
                overlaps_at_join(from, start, end)
            } else {
                intersects(start, end, from, to)
            }
        };
        if segments[i + 1..].iter().copied().any(conflicts)
            || (0..n)
                .filter(|&x| x != a)
                .any(|x| conflicts((x, (x + 1) % n)))
        {
            return false;
        }
    }
    true
}
