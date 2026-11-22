//! FabricCraft's headless geometry domain.

pub mod develop;
pub mod geometry;
pub mod planar;
pub mod tube;

/// Returns the current prototype schema version.
pub fn schema_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_is_versioned() {
        assert_eq!(super::schema_version(), 1);
    }
}
