use env::Point;
use std::collections::BTreeSet;
use std::fmt;

/// A region is a set of points where, within any given basic block,
/// the points must be continuous. We represent this as a map:
///
///     B -> start..end
///
/// where `B` is a basic block identifier and start/end are indices.
#[derive(Clone, PartialEq, Eq)]
pub struct Region {
    points: BTreeSet<Point>
}

impl Region {
    pub fn new() -> Self {
        Region { points: BTreeSet::new() }
    }

    pub fn add_point(&mut self, point: Point) -> bool {
        self.points.insert(point)
    }

    pub fn add_region(&mut self, region: &Region) -> bool {
        let l = self.points.len();
        self.points.extend(&region.points);
        l != self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn contains(&self, point: Point) -> bool {
        self.points.contains(&point)
    }
}

impl fmt::Debug for Region {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{{")?;
        for (index, point) in self.points.iter().enumerate() {
            if index > 0 {
                write!(fmt, ", ")?;
            }
            write!(fmt, "{:?}", point)?;
        }
        write!(fmt, "}}")?;
        Ok(())
    }
}
