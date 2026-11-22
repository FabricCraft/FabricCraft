use std::fmt;

use super::{MAX_COORDINATE_MM, PlanarPanel, intersects, validate_outline};

/// Full-scale cut and sewing lines sharing an origin at the cut line's bounds.
#[derive(Debug)]
pub struct CutPattern {
    sewing: Vec<[f64; 2]>,
    cut: Vec<[f64; 2]>,
    size: [f64; 2],
    allowance_mm: f64,
}

impl CutPattern {
    /// Offset the outside of a single sewing outline, with mitered corners.
    /// Reject allowances that require changing the contour's topology.
    pub fn new(panel: &PlanarPanel, allowance_mm: f64) -> Result<Self, AllowanceError> {
        if !allowance_mm.is_finite() || allowance_mm < 0. {
            return Err(AllowanceError::InvalidDistance);
        }
        let sewing = panel.outline_mm();
        if allowance_mm == 0. {
            return Ok(Self {
                sewing: sewing.to_vec(),
                cut: sewing.to_vec(),
                size: panel.size_mm(),
                allowance_mm,
            });
        }
        let edges: Vec<_> = loop_edges(sewing).collect();
        let outward: Vec<_> = edges
            .iter()
            .map(|&([x, y], [u, v])| {
                let [dx, dy] = [u - x, v - y];
                let length = dx.hypot(dy);
                [dy / length, -dx / length]
            })
            .collect();
        let n = sewing.len();
        let cut: Vec<_> = sewing
            .iter()
            .enumerate()
            .map(|(i, &[x, y])| {
                // The sum of the adjacent outward normals gives the miter
                // direction. Its dot product with either normal fixes the
                // distance from the corresponding sewing edge.
                let [a, b] = outward[(i + n - 1) % n];
                let [c, d] = outward[i];
                let [u, v] = [a + c, b + d];
                let scale = 2. * allowance_mm / (u * u + v * v);
                [x + u * scale, y + v * scale]
            })
            .collect();
        if cut
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)
        {
            return Err(AllowanceError::CoordinateRange);
        }
        validate_outline(&cut).map_err(|_| AllowanceError::Overlap)?;
        for ((a, b), [x, y]) in loop_edges(&cut).zip(outward) {
            // A narrowing notch can collapse or reverse an edge before the
            // offset can still be described by the same contour.
            let along = (b[0] - a[0]) * -y + (b[1] - a[1]) * x;
            if along <= 0. || edges.iter().any(|&(c, d)| intersects(a, b, c, d)) {
                return Err(AllowanceError::Overlap);
            }
        }
        let [min_x, min_y] = [
            cut.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
            cut.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
        ];
        let translate = |[x, y]: [f64; 2]| [x - min_x, y - min_y];
        Ok(Self {
            sewing: sewing.iter().copied().map(translate).collect(),
            size: [
                cut.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max) - min_x,
                cut.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max) - min_y,
            ],
            cut: cut.into_iter().map(translate).collect(),
            allowance_mm,
        })
    }

    pub fn sewing_line_mm(&self) -> &[[f64; 2]] {
        &self.sewing
    }

    pub fn cut_line_mm(&self) -> &[[f64; 2]] {
        &self.cut
    }

    pub fn size_mm(&self) -> [f64; 2] {
        self.size
    }

    pub fn allowance_mm(&self) -> f64 {
        self.allowance_mm
    }
}

fn loop_edges(points: &[[f64; 2]]) -> impl Iterator<Item = ([f64; 2], [f64; 2])> {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
}

#[derive(Debug)]
pub enum AllowanceError {
    InvalidDistance,
    CoordinateRange,
    Overlap,
}

impl fmt::Display for AllowanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDistance => "Enter a finite seam allowance of zero or more.",
            Self::CoordinateRange => "The seam allowance produces a cut line outside the supported coordinate range.",
            Self::Overlap => "The seam allowance overlaps the outline or closes a narrow gap. Reduce the allowance or adjust the boundary.",
        })
    }
}

impl std::error::Error for AllowanceError {}
