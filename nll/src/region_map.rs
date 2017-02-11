#![allow(dead_code)]

use env::Point;
use nll_repr::repr;
use region::Region;
use std::collections::HashMap;

pub struct RegionMap {
    num_vars: usize,

    // P \in R
    in_constraints: Vec<RegionVariable>,

    // Rq <= PQ(Rp)
    //
    // Effectively, if Rq non-empty, Rp += P + Rp
    //
    // Used for regions in invariant or contravariant positions.
    //
    // Note that P and Q are implied by the region variable names themselves.
    neg_flow_constraints: Vec<(/*P*/ RegionVariable, /*Q*/ RegionVariable)>,

    // Rq >= PQ(Rp)
    //
    // Have to compute PQ(Vp.read), which is "all points in Vp.read
    // that are reachable from Q without exiting Vp.read".
    //
    // Used for regions in invariant or covariant positions.
    pos_flow_constraints: Vec<(RegionVariable, RegionVariable)>,

    // Rp <= Rq
    //
    // Used for subtype relations within one point.
    subregion_constraints: Vec<(RegionVariable, RegionVariable)>,

    // Check whether a given variable ultimately contains a given point
    assertions: Vec<(RegionVariable, Point, bool)>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegionVariable {
    name: repr::Variable,
    point: Point,
    index: usize,
}

const READ: usize = 0;
const WRITE: usize = 1;

impl RegionMap {
    pub fn new() -> Self {
        RegionMap {
            num_vars: 0,
            in_constraints: vec![],
            neg_flow_constraints: vec![],
            pos_flow_constraints: vec![],
            subregion_constraints: vec![],
            assertions: vec![],
        }
    }

    pub fn read_var(&mut self, var: repr::Variable, point: Point) {
        log!("read_var({:?}, {:?})", var, point);
        let rv = RegionVariable { name: var, point: point, index: READ };
        self.in_constraints.push(rv);
    }

    pub fn write_var(&mut self, var: repr::Variable, point: Point) {
        log!("write_var({:?}, {:?})", var, point);
        let rv = RegionVariable { name: var, point: point, index: WRITE };
        self.in_constraints.push(rv);
        self.read_var(var, point); // `read >= write` always holds
    }

    pub fn neg_flow(&mut self, v: repr::Variable, p: Point, q: Point) {
        log!("neg_flow({:?}, {:?}, {:?})", v, p, q);
        let rp = RegionVariable { name: v, point: p, index: READ };
        let rq = RegionVariable { name: v, point: q, index: READ };
        self.neg_flow_constraints.push((rp, rq));
    }

    pub fn pos_flow(&mut self, v: repr::Variable, p: Point, q: Point) {
        log!("pos_flow({:?}, {:?}, {:?})", v, p, q);
        let rp = RegionVariable { name: v, point: p, index: READ };
        let rq = RegionVariable { name: v, point: q, index: READ };
        self.pos_flow_constraints.push((rp, rq));
    }

    pub fn subregion(&mut self, v: repr::Variable, w: repr::Variable, p: Point) {
        log!("subregion({:?}, {:?}, {:?})", v, w, p);
        for &index in &[READ, WRITE] {
            let rv = RegionVariable { name: v, point: p, index: index };
            let rw = RegionVariable { name: w, point: p, index: index };
            self.subregion_constraints.push((rv, rw));
        }
    }

    pub fn assert_region_contains(&mut self,
                                  region_name: repr::Variable,
                                  region_point: Point,
                                  region_index: usize,
                                  point: Point,
                                  expected: bool) {
        let rv = RegionVariable { name: region_name, point: region_point, index: region_index };
        self.assertions.push((rv, point, expected));
    }

    pub fn solve<'m>(&'m self) -> RegionSolution<'m> {
        RegionSolution::new(self)
    }
}

pub struct RegionSolution<'m> {
    region_map: &'m RegionMap,
    values: HashMap<RegionVariable, Region>,
}

impl<'m> RegionSolution<'m> {
    pub fn new(region_map: &'m RegionMap) -> Self {
        let mut solution = RegionSolution {
            region_map: region_map,
            values: HashMap::new(),
        };
        solution.find();
        solution
    }

    fn region_mut(&mut self, rv: RegionVariable) -> &mut Region {
        self.values.entry(rv).or_insert_with(|| Region::new())
    }

    fn region(&self, rv: RegionVariable) -> Region {
        self.values.get(&rv)
                   .cloned()
                   .unwrap_or(Region::new())
    }

    fn find(&mut self) {
        for &var in &self.region_map.in_constraints {
            self.region_mut(var).add_point(var.point);
        }

        let mut changed = true;
        while changed {
            changed = false;

            for &(p, q) in &self.region_map.neg_flow_constraints {
                let r_q = self.region(q);
                if r_q.is_empty() {
                    continue;
                }

                changed |= self.region_mut(p).add_region(&r_q);
                changed |= self.region_mut(p).add_point(p.point);
            }

            assert!(self.region_map.pos_flow_constraints.is_empty());

            for &(p, q) in &self.region_map.subregion_constraints {
                let p = self.region(p);
                changed |= self.region_mut(q).add_region(&p);
            }
        }
    }

    pub fn check(&self) -> usize {
        let mut errors = 0;

        for &(rv, point, expected) in &self.region_map.assertions {
            let actual_region = self.region(rv);
            let contained = actual_region.contains(point);
            if expected && !contained {
                println!("error: region `{:?}` did not contain `{:?}` as it should have",
                         rv, point);
                println!("    actual region `{:?}`", actual_region);
                errors += 1;
            } else if !expected && contained {
                println!("error: region `{:?}` contained `{:?}`, which it should not have",
                         rv, point);
                println!("    actual region `{:?}`", actual_region);
                errors += 1;
            } else {
                assert_eq!(expected, contained);
            }
        }

        errors
    }
}
