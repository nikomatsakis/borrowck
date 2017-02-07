use graph::BasicBlockIndex;
use env::Point;
use std::collections::BTreeMap;
use std::cmp;
use std::fmt;

/// A region is a set of points where, within any given basic block,
/// the points must be continuous. We represent this as a map:
///
///     B -> start..end
///
/// where `B` is a basic block identifier and start/end are indices.
#[derive(Clone, PartialEq, Eq)]
pub struct Region {
    ranges: BTreeMap<BasicBlockIndex, ActionRange>,
}

impl Region {
    pub fn new() -> Self {
        Region { ranges: BTreeMap::new() }
    }

    pub fn add_point(&mut self, point: Point) -> bool {
        let range = self.ranges.entry(point.block).or_insert(ActionRange::new());
        range.add(point.action)
    }

    pub fn add_region(&mut self, region: &Region) -> bool {
        let mut result = false;
        for (&block, range) in &region.ranges {
            if !range.is_empty() {
                let start = Point { block: block, action: range.start };
                let end = Point { block: block, action: range.end - 1 };
                result |= self.add_point(start);
                result |= self.add_point(end);
            }
        }
        result
    }

    pub fn contains(&self, point: Point) -> bool {
        let result = self.ranges.get(&point.block)
                                .unwrap_or(&ActionRange::new())
                                .contains(point.action);
        log!("contains(self={:?}, point={:?}) = {}", self, point, result);
        result
    }
}

impl fmt::Debug for Region {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{{")?;
        for (index, (&block, range)) in self.ranges.iter().enumerate() {
            if index > 0 { write!(fmt, ", ")?; }
            write!(fmt, "{:?}/{:?}", block, range)?;
        }
        write!(fmt, "}}")?;
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ActionRange {
    start: usize,
    end: usize
}

impl ActionRange {
    pub fn new() -> ActionRange {
        ActionRange { start: 0, end: 0 }
    }

    pub fn add(&mut self, i: usize) -> bool {
        if self.is_empty() {
            self.start = i;
            self.end = i + 1;
            true
        } else {
            let (start, end) = (self.start, self.end);
            self.start = cmp::min(i, start);
            self.end = cmp::max(i+1, end);
            start != self.start || end != self.end
        }
    }

    pub fn contains(&self, i: usize) -> bool {
        (i >= self.start) && (i < self.end)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Debug for ActionRange {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "[{:?}..{:?}]", self.start, self.end)
    }
}

