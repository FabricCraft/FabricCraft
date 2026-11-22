//! Open an annular reference patch along a seam selected in FabricCraft.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
    fmt,
};

use crate::{
    develop::{DevelopError, DevelopedPanel, distance_2d, distance_tolerance},
    geometry::{Face, GeometryError, Surface, Vertex, VertexId, face_neighbors},
    planar::{MAX_COORDINATE_MM, norm, sub},
};

type Edge = (VertexId, VertexId);
type Corner = (usize, usize);
type EdgeGraph = BTreeMap<VertexId, Vec<(VertexId, f64)>>;

#[derive(Debug)]
pub struct TubeDevelopment {
    development: DevelopedPanel,
    source_path: Vec<VertexId>,
    source_length_mm: f64,
    banks_mm: [Vec<[f64; 2]>; 2],
    max_paired_edge_error_mm: f64,
}

impl TubeDevelopment {
    /// Choose an explicit path of source vertices, or the shortest mesh-edge
    /// path connecting the two rims. The reference surface stays intact.
    pub fn from_surface(surface: &Surface, path: Option<&[VertexId]>) -> Result<Self, TubeError> {
        let record = surface.record();
        let positions: BTreeMap<_, _> = record
            .vertices_mm
            .iter()
            .map(|v| (v.id, v.position))
            .collect();
        let mut directed = BTreeMap::new();
        for (face, triangle) in record.faces.iter().enumerate() {
            for (slot, edge) in face_edges(triangle).into_iter().enumerate() {
                directed.insert(edge, (face, slot));
            }
        }
        let boundaries: BTreeMap<_, _> = directed
            .keys()
            .filter(|&&(a, b)| !directed.contains_key(&(b, a)))
            .copied()
            .collect();
        let undirected_edge_count = (directed.len() + boundaries.len()) / 2;
        let [rim_a, rim_b] = boundary_rims(boundaries)?;
        let neighbors = face_neighbors(&record.faces);
        let mut pending = vec![0];
        let mut reached = vec![false; record.faces.len()];
        while let Some(face) = pending.pop() {
            if !std::mem::replace(&mut reached[face], true) {
                pending.extend(neighbors[face].iter().flatten().map(|&(other, _)| other));
            }
        }
        let used: BTreeSet<_> = directed.keys().map(|&(a, _)| a).collect();
        // A connected orientable manifold with two rims and Euler characteristic zero is an annulus.
        if reached.contains(&false) || used.len() + record.faces.len() != undirected_edge_count {
            return Err(TubeError::NotATube);
        }
        if used
            .iter()
            .any(|v| positions[v].iter().any(|x| x.abs() > MAX_COORDINATE_MM))
        {
            return Err(TubeError::CoordinateRange);
        }

        let source_path = match path {
            Some(path) => path.to_vec(),
            None => {
                let mut graph = EdgeGraph::new();
                for &(a, b) in directed.keys() {
                    if a < b || !directed.contains_key(&(b, a)) {
                        let length = norm(sub(positions[&a], positions[&b]));
                        graph.entry(a).or_default().push((b, length));
                        graph.entry(b).or_default().push((a, length));
                    }
                }
                shortest_seam(&graph, &rim_a, &rim_b).ok_or(TubeError::NotATube)?
            }
        };
        validate_path(&source_path, &rim_a, &rim_b, &directed)?;
        let source_length_mm = source_path
            .windows(2)
            .map(|w| norm(sub(positions[&w[0]], positions[&w[1]])))
            .sum();
        let (opened, banks) = split_seam(surface, &source_path, &directed, &neighbors, &positions)?;
        let development = DevelopedPanel::from_surface(&opened).map_err(TubeError::Develop)?;
        let outline = development.panel();
        let flat: BTreeMap<_, _> = outline
            .outline_vertex_ids()
            .iter()
            .copied()
            .zip(outline.outline_mm().iter().copied())
            .collect();
        let [a, b] = banks.map(|bank| {
            bank.into_iter()
                .map(|id| flat.get(&id).copied())
                .collect::<Option<Vec<_>>>()
        });
        let (Some(a), Some(b)) = (a, b) else {
            return Err(TubeError::InvalidSplit);
        };
        let mut max_paired_edge_error_mm: f64 = 0.;
        for (a_pair, b_pair) in a.windows(2).zip(b.windows(2)) {
            let error =
                (distance_2d(a_pair[0], a_pair[1]) - distance_2d(b_pair[0], b_pair[1])).abs();
            let tolerance =
                distance_tolerance(a_pair[0], a_pair[1]) + distance_tolerance(b_pair[0], b_pair[1]);
            if error > tolerance {
                return Err(TubeError::Mismatch(error));
            }
            max_paired_edge_error_mm = max_paired_edge_error_mm.max(error);
        }
        Ok(Self {
            development,
            source_path,
            source_length_mm,
            banks_mm: [a, b],
            max_paired_edge_error_mm,
        })
    }

    pub fn development(&self) -> &DevelopedPanel {
        &self.development
    }
    pub fn source_path(&self) -> &[VertexId] {
        &self.source_path
    }
    pub fn source_length_mm(&self) -> f64 {
        self.source_length_mm
    }
    pub fn banks_mm(&self) -> [&[[f64; 2]]; 2] {
        self.banks_mm.each_ref().map(Vec::as_slice)
    }
    pub fn max_paired_edge_error_mm(&self) -> f64 {
        self.max_paired_edge_error_mm
    }
}

fn face_edges(face: &Face) -> [Edge; 3] {
    let [a, b, c] = face.vertices;
    [(a, b), (b, c), (c, a)]
}

fn boundary_rims(
    mut edges: BTreeMap<VertexId, VertexId>,
) -> Result<[BTreeSet<VertexId>; 2], TubeError> {
    let mut rims = [BTreeSet::new(), BTreeSet::new()];
    for rim in &mut rims {
        let start = *edges.keys().next().ok_or(TubeError::NotATube)?;
        let mut vertex = start;
        loop {
            rim.insert(vertex);
            vertex = edges.remove(&vertex).ok_or(TubeError::NotATube)?;
            if vertex == start {
                break;
            }
        }
    }
    if !edges.is_empty() {
        return Err(TubeError::NotATube);
    }
    Ok(rims)
}

fn validate_path(
    path: &[VertexId],
    a: &BTreeSet<VertexId>,
    b: &BTreeSet<VertexId>,
    edges: &BTreeMap<Edge, Corner>,
) -> Result<(), TubeError> {
    let [start, interior @ .., end] = path else {
        return Err(TubeError::InvalidPath);
    };
    let connects_rims =
        (a.contains(start) && b.contains(end)) || (b.contains(start) && a.contains(end));
    if !connects_rims
        || interior.iter().any(|id| a.contains(id) || b.contains(id))
        || path.iter().collect::<BTreeSet<_>>().len() != path.len()
        || path
            .windows(2)
            .any(|w| !edges.contains_key(&(w[0], w[1])) || !edges.contains_key(&(w[1], w[0])))
    {
        return Err(TubeError::InvalidPath);
    }
    Ok(())
}

fn shortest_seam(
    graph: &EdgeGraph,
    start: &BTreeSet<VertexId>,
    target: &BTreeSet<VertexId>,
) -> Option<Vec<VertexId>> {
    let mut queue = BinaryHeap::new();
    let mut paths = BTreeMap::new();
    for &vertex in start {
        paths.insert(vertex, (0., None));
        queue.push(Candidate { cost: 0., vertex });
    }
    while let Some(Candidate { cost, vertex }) = queue.pop() {
        if cost > paths[&vertex].0 {
            continue;
        }
        if target.contains(&vertex) {
            let mut path = vec![vertex];
            let mut cursor = vertex;
            while let Some(previous) = paths[&cursor].1 {
                path.push(previous);
                cursor = previous;
            }
            path.reverse();
            return Some(path);
        }
        for &(neighbor, length) in &graph[&vertex] {
            let next = cost + length;
            if paths.get(&neighbor).is_none_or(|&(cost, _)| next < cost) {
                paths.insert(neighbor, (next, Some(vertex)));
                queue.push(Candidate {
                    cost: next,
                    vertex: neighbor,
                });
            }
        }
    }
    None
}

struct Candidate {
    cost: f64,
    vertex: VertexId,
}
impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.vertex.cmp(&self.vertex))
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn split_seam(
    surface: &Surface,
    path: &[VertexId],
    directed: &BTreeMap<Edge, Corner>,
    neighbors: &[[Option<Corner>; 3]],
    positions: &BTreeMap<VertexId, [f64; 3]>,
) -> Result<(Surface, [Vec<VertexId>; 2]), TubeError> {
    let record = surface.record();
    let ordered = |a: VertexId, b: VertexId| (a.min(b), a.max(b));
    let cuts: BTreeSet<_> = path.windows(2).map(|w| ordered(w[0], w[1])).collect();
    // Each corner starts independently. Non-seam neighbors identify corresponding corners.
    let mut roots: Vec<_> = (0..3 * record.faces.len()).collect();
    for (face, adjacent) in neighbors.iter().enumerate() {
        let edges = face_edges(&record.faces[face]);
        for (slot, neighbor) in adjacent.iter().enumerate() {
            if let Some((other, rev)) = *neighbor {
                let (a, b) = edges[slot];
                if other > face && !cuts.contains(&ordered(a, b)) {
                    join_corners(&mut roots, 3 * face + slot, 3 * other + (rev + 1) % 3);
                    join_corners(&mut roots, 3 * face + (slot + 1) % 3, 3 * other + rev);
                }
            }
        }
    }
    let mut new_record = record.clone();
    let mut root_ids = vec![None; roots.len()];
    let mut kept = BTreeSet::new();
    let mut available = (0..=u64::MAX)
        .map(VertexId)
        .filter(|id| !positions.contains_key(id));
    for (face, triangle) in new_record.faces.iter_mut().enumerate() {
        for (slot, id) in triangle.vertices.iter_mut().enumerate() {
            let root = find_corner(&mut roots, 3 * face + slot);
            if let Some(known) = root_ids[root] {
                *id = known;
                continue;
            }
            if !kept.insert(*id) {
                let duplicate = available.next().ok_or(TubeError::InvalidSplit)?;
                new_record.vertices_mm.push(Vertex {
                    id: duplicate,
                    position: positions[id],
                });
                *id = duplicate;
            }
            root_ids[root] = Some(*id);
        }
    }

    let mut banks = [Vec::new(), Vec::new()];
    for step in path.windows(2) {
        let (a, b) = (step[0], step[1]);
        let (first, fwd) = directed[&(a, b)];
        let (second, rev) = directed[&(b, a)];
        let u = new_record.faces[first].vertices;
        let v = new_record.faces[second].vertices;
        let pairs = [[u[fwd], u[(fwd + 1) % 3]], [v[(rev + 1) % 3], v[rev]]];
        if pairs[0][0] == pairs[1][0] || pairs[0][1] == pairs[1][1] {
            return Err(TubeError::InvalidSplit);
        }
        for (bank, [start, end]) in banks.iter_mut().zip(pairs) {
            match bank.last() {
                None => bank.push(start),
                Some(&last) if last == start => {}
                _ => return Err(TubeError::InvalidSplit),
            }
            bank.push(end);
        }
    }
    let cut = Surface::from_record(new_record).map_err(TubeError::Geometry)?;
    Ok((cut, banks))
}

fn find_corner(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

fn join_corners(parent: &mut [usize], a: usize, b: usize) {
    let a = find_corner(parent, a);
    let b = find_corner(parent, b);
    parent[a.max(b)] = a.min(b);
}

#[derive(Debug)]
pub enum TubeError {
    NotATube,
    CoordinateRange,
    InvalidPath,
    InvalidSplit,
    Geometry(GeometryError),
    Develop(DevelopError),
    Mismatch(f64),
}

impl fmt::Display for TubeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotATube => write!(
                f,
                "select one connected tube side with two open rims and shared mesh vertices"
            ),
            Self::CoordinateRange => write!(
                f,
                "panel coordinates must lie within ±{MAX_COORDINATE_MM} mm"
            ),
            Self::InvalidPath => write!(
                f,
                "choose a continuous path of mesh edges from one rim to the other without revisiting a vertex or following either rim"
            ),
            Self::InvalidSplit => write!(
                f,
                "the selected seam did not produce two corresponding outline edges"
            ),
            Self::Geometry(error) => write!(f, "seam cut produced invalid geometry: {error}"),
            Self::Develop(error) => write!(f, "{error}"),
            Self::Mismatch(mm) => write!(f, "paired seam edges differ by {mm:.6e} mm"),
        }
    }
}

impl std::error::Error for TubeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Geometry(error) => Some(error),
            Self::Develop(error) => Some(error),
            _ => None,
        }
    }
}
