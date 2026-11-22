//! In-memory triangular surfaces in right-handed, Z-up millimetres.
//!
//! ```
//! use fabriccraft_core::geometry::{
//!     Face, FaceId, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId, Vertex, VertexId,
//! };
//! let source = Provenance {
//!     source: "triangle".into(),
//!     unit: LengthUnit::Centimetre,
//!     frame: ImportFrame::IDENTITY,
//! };
//! let vertices = [[0., 0., 0.], [2., 0., 0.], [0., 1., 0.]]
//!     .into_iter().zip(1..)
//!     .map(|(position, id)| Vertex { id: VertexId(id), position }).collect();
//! let face = Face { id: FaceId(1), vertices: [1, 2, 3].map(VertexId) };
//! let surface = Surface::import(SurfaceId(1), source, vertices, vec![face])?;
//! assert_eq!(surface.record().vertices_mm[1].position, [20., 0., 0.]);
//! # Ok::<(), fabriccraft_core::geometry::GeometryError>(())
//! ```

mod adjacency;
mod validation;

pub(crate) use adjacency::face_neighbors;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceId(pub u64);

/// Stable vertex identity within a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VertexId(pub u64);

/// Stable face identity within a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthUnit {
    Millimetre,
    Centimetre,
    Metre,
    Inch,
}

impl LengthUnit {
    pub fn millimetres_per_unit(self) -> f64 {
        match self {
            Self::Millimetre => 1.,
            Self::Centimetre => 10.,
            Self::Metre => 1000.,
            Self::Inch => 25.4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
    NegativeX,
    NegativeY,
    NegativeZ,
}

impl Axis {
    fn component(self) -> (usize, f64) {
        match self {
            Self::X => (0, 1.),
            Self::Y => (1, 1.),
            Self::Z => (2, 1.),
            Self::NegativeX => (0, -1.),
            Self::NegativeY => (1, -1.),
            Self::NegativeZ => (2, -1.),
        }
    }
}

/// Each canonical axis selects one signed source axis. Translation is applied last.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImportFrame {
    pub axes: [Axis; 3],
    pub translation_mm: [f64; 3],
}

impl ImportFrame {
    pub const IDENTITY: Self = Self {
        axes: [Axis::X, Axis::Y, Axis::Z],
        translation_mm: [0.; 3],
    };

    fn reverses_winding(&self) -> Result<bool, GeometryError> {
        let components = self.axes.map(Axis::component);
        let [a, b, c] = components.map(|(index, _)| index);
        if a == b || a == c || b == c || !self.translation_mm.iter().all(|x| x.is_finite()) {
            return Err(GeometryError::InvalidFrame);
        }
        let odd_permutation = (a > b) ^ (a > c) ^ (b > c);
        let odd_reflection = components.iter().filter(|(_, sign)| *sign < 0.).count() % 2 == 1;
        Ok(odd_permutation ^ odd_reflection)
    }

    fn position_mm(&self, position: [f64; 3], scale: f64) -> [f64; 3] {
        std::array::from_fn(|i| {
            let (index, sign) = self.axes[i].component();
            (position[index] * scale) * sign + self.translation_mm[i]
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    pub source: String,
    pub unit: LengthUnit,
    pub frame: ImportFrame,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub id: VertexId,
    /// Source units on import and millimetres in a canonical record.
    pub position: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    pub id: FaceId,
    /// The ordered IDs define the triangle's front side.
    pub vertices: [VertexId; 3],
}

/// Versioned interchange record. Positions and winding are already canonical.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceRecord {
    pub schema_version: u32,
    pub id: SurfaceId,
    pub provenance: Provenance,
    pub vertices_mm: Vec<Vertex>,
    pub faces: Vec<Face>,
}

/// A validated triangle surface. Open boundaries and unused vertices are allowed.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface(SurfaceRecord);

impl Surface {
    /// Convert from declared source units and frame, then validate the resulting surface.
    pub fn import(
        id: SurfaceId,
        provenance: Provenance,
        mut vertices: Vec<Vertex>,
        mut faces: Vec<Face>,
    ) -> Result<Self, GeometryError> {
        let reverse = provenance.frame.reverses_winding()?;
        let scale = provenance.unit.millimetres_per_unit();
        for vertex in &mut vertices {
            vertex.position = provenance.frame.position_mm(vertex.position, scale);
        }
        if reverse {
            for face in &mut faces {
                face.vertices.swap(1, 2);
            }
        }
        Self::validated(SurfaceRecord {
            schema_version: crate::schema_version(),
            id,
            provenance,
            vertices_mm: vertices,
            faces,
        })
    }

    /// Validate a canonical record, preserving its positions and winding. Records
    /// emitted by either constructor have vertices and faces sorted by stable ID.
    pub fn from_record(record: SurfaceRecord) -> Result<Self, GeometryError> {
        if record.schema_version != crate::schema_version() {
            return Err(GeometryError::UnsupportedVersion(record.schema_version));
        }
        record.provenance.frame.reverses_winding()?;
        Self::validated(record)
    }

    fn validated(mut record: SurfaceRecord) -> Result<Self, GeometryError> {
        record.vertices_mm.sort_unstable_by_key(|v| v.id);
        record.faces.sort_unstable_by_key(|f| f.id);
        validation::validate(&record)?;
        Ok(Self(record))
    }

    pub fn record(&self) -> &SurfaceRecord {
        &self.0
    }

    pub fn into_record(self) -> SurfaceRecord {
        self.0
    }
}

/// First structural error in deterministic validation order. IDs refer to the input surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    UnsupportedVersion(u32),
    MissingSource,
    InvalidFrame,
    EmptySurface,
    DuplicateVertex(VertexId),
    DuplicateFace(FaceId),
    NonFinitePosition(VertexId),
    MissingVertex {
        face: FaceId,
        vertex: VertexId,
    },
    RepeatedCorner(FaceId),
    /// The doubled face area divided by the longest edge length squared is at
    /// or below `f64::EPSILON` (dimensionless).
    DegenerateFace(FaceId),
    DuplicateTriangle(FaceId, FaceId),
    NonManifoldEdge([VertexId; 2]),
    InconsistentWinding([VertexId; 2]),
    DisconnectedFan(VertexId),
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "surface validation failed: {self:?}")
    }
}

impl std::error::Error for GeometryError {}
