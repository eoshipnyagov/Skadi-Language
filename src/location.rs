// v01/src/location.rs
/// Represents a single location in the source file.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl Location {
    pub fn new(line: u32, col: u32) -> Self {
        Location { line, column: col }
    }
}

impl Default for Location {
    fn default() -> Self {
        Self::new(1, 1)
    }
}