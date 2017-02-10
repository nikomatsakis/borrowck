#![allow(dead_code)]

use env::Point;
use nll_repr::repr;
use region::Region;
use std::collections::HashMap;

pub struct RegionMap {
    num_vars: usize,
    in_constraints: Vec<(RegionVariable, Point)>,
    flow_constraints: Vec<(RegionVariable, Point, Point)>,
    outlive_constraints: Vec<(RegionVariable, RegionVariable)>,
    user_region_names: HashMap<repr::RegionName, RegionVariable>,
    region_eq_assertions: Vec<(repr::RegionName, Region)>,
    region_in_assertions: Vec<(repr::RegionName, Point, bool)>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RefTy {
    pub read: RegionVariable,
    pub write: RegionVariable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionVariable {
    index: usize,
}

pub struct UseConstraint {
    var: RegionVariable,
    contains: Point,
}

pub struct InAssertion {
    var: RegionVariable,
    contains: Point,
}

pub struct OutAssertion {
    var: RegionVariable,
    contains: Point,
}

impl RegionMap {
    pub fn new() -> Self {
        RegionMap {
            num_vars: 0,
            in_constraints: vec![],
            flow_constraints: vec![],
            outlive_constraints: vec![],
            user_region_names: HashMap::new(),
            region_eq_assertions: vec![],
            region_in_assertions: vec![],
        }
    }

    pub fn new_var(&mut self) -> RegionVariable {
        self.num_vars += 1;
        RegionVariable { index: self.num_vars - 1 }
    }

    pub fn fresh_ty(&mut self) -> RefTy {
        let ty = RefTy {
            read: self.new_var(),
            write: self.new_var(),
        };
        self.outlive_constraints.push((ty.read, ty.write));
        ty
    }

    pub fn read_ref(&mut self, ty: RefTy, point: Point) {
        self.in_constraints.push((ty.read, point));
    }

    pub fn write_ref(&mut self, ty: RefTy, point: Point) {
        self.in_constraints.push((ty.write, point));
    }

    pub fn user_names(&mut self, read: repr::RegionName, write: repr::RegionName, ty: RefTy) {
        self.user_region_names.insert(read, ty.read);
        self.user_region_names.insert(write, ty.write);
        log!("user_names: read {:?}={:?}", read, ty.read);
        log!("user_names: write {:?}={:?}", write, ty.write);
    }

    pub fn assert_region_eq(&mut self, name: repr::RegionName, region: Region) {
        self.region_eq_assertions.push((name, region));
    }

    pub fn assert_region_contains(&mut self,
                                  name: repr::RegionName,
                                  point: Point,
                                  expected: bool) {
        self.region_in_assertions.push((name, point, expected));
    }

    pub fn flow(&mut self, ty: RefTy, a_point: Point, b_point: Point) {
        self.flow_constraints.push((ty.read, a_point, b_point));
    }

    /// Create the constraints such that `sub_ty <: super_ty`. Here we
    /// assume that both types are instantiations of a common 'erased
    /// type skeleton', and hence that the regions we will encounter
    /// as we iterate line up prefectly.
    ///
    /// We also assume all regions are contravariant for the time
    /// being.
    pub fn subtype(&mut self, a_ty: RefTy, b_ty: RefTy) {
        self.outlive_constraints.push((a_ty.read, b_ty.read));
        self.outlive_constraints.push((a_ty.write, b_ty.write));
    }

    pub fn solve<'m>(&'m self) -> RegionSolution<'m> {
        RegionSolution::new(self)
    }
}

pub struct RegionSolution<'m> {
    region_map: &'m RegionMap,
    values: Vec<Region>,
}

impl<'m> RegionSolution<'m> {
    pub fn new(region_map: &'m RegionMap) -> Self {
        let mut solution = RegionSolution {
            region_map: region_map,
            values: (0..region_map.num_vars).map(|_| Region::new()).collect(),
        };
        solution.find();
        solution
    }

    fn find(&mut self) {
        for &(var, point) in &self.region_map.in_constraints {
            self.values[var.index].add_point(point);
            log!("user_constraints: var={:?} value={:?} point={:?}",
                 var,
                 self.values[var.index],
                 point);
        }

        let mut changed = true;
        while changed {
            changed = false;

            // Data in region R flows from point A to point B (without changing
            // name). Therefore, if it is used in B, A must in R.
            for &(a, a_point, b_point) in &self.region_map.flow_constraints {
                if self.values[a.index].contains(b_point) {
                    changed |= self.values[a.index].add_point(a_point);
                }
            }

            // 'a: 'b -- add everything 'b into 'a
            for &(a, b) in &self.region_map.outlive_constraints {
                assert!(a != b);

                log!("outlive_constraints: a={:?} a_value={:?}",
                     a,
                     self.values[a.index]);
                log!("                       b={:?} b_value={:?}",
                     b,
                     self.values[b.index]);

                // In any case, A must include all points in B.
                let b_value = self.values[b.index].clone();
                changed |= self.values[a.index].add_region(&b_value);
            }
        }
    }

    pub fn region(&self, var: RegionVariable) -> &Region {
        &self.values[var.index]
    }

    pub fn check(&self) -> usize {
        let mut errors = 0;

        for &(user_region, ref expected_region) in &self.region_map.region_eq_assertions {
            let region_var = self.region_map.user_region_names[&user_region];
            let actual_region = self.region(region_var);
            if actual_region != expected_region {
                println!("error: region `{:?}` came to `{:?}`, which was not expected",
                         user_region,
                         actual_region);
                println!("    expected `{:?}`", expected_region);
                errors += 1;
            }
        }

        for &(user_region, point, expected) in &self.region_map.region_in_assertions {
            let region_var = self.region_map.user_region_names[&user_region];
            let actual_region = self.region(region_var);
            let contained = actual_region.contains(point);
            if expected && !contained {
                println!("error: region `{:?}` did not contain `{:?}` as it should have",
                         user_region, point);
                println!("    actual region `{:?}`", actual_region);
                errors += 1;
            } else if !expected && contained {
                println!("error: region `{:?}` contained `{:?}`, which it should not have",
                         user_region, point);
                println!("    actual region `{:?}`", actual_region);
            } else {
                assert_eq!(expected, contained);
            }
        }

        errors
    }
}
