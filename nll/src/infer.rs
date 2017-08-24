use env::{Environment, Point};
use region::Region;
use std::collections::HashSet;

pub struct InferenceContext {
    /// for each region variable, sets of points where live data in
    /// the region exists
    values: Vec<Region>,
    constraints: Vec<Constraint>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegionVariable {
    index: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Constraint {
    sub: RegionVariable,
    sup: RegionVariable,
    point: Point,
}

impl InferenceContext {
    pub fn new() -> Self {
        InferenceContext {
            values: vec![],
            constraints: vec![],
        }
    }

    pub fn add_var(&mut self) -> RegionVariable {
        let index = self.values.len();
        self.values.push(Region::new());
        RegionVariable { index }
    }

    pub fn add_live_point(&mut self, v: RegionVariable, point: Point) {
        log!("add_live_point({:?}, {:?})", v, point);
        self.values[v.index].add_point(point);
    }

    pub fn add_outlives(&mut self, sup: RegionVariable, sub: RegionVariable, point: Point) {
        log!("add_outlives({:?}: {:?} @ {:?})", sup, sub, point);
        self.constraints.push(Constraint { sup, sub, point });
    }

    pub fn region(&self, v: RegionVariable) -> &Region {
        &self.values[v.index]
    }

    pub fn solve(&mut self, env: &Environment) {
        let mut changed = true;
        let mut dfs = Dfs::new(env);
        while changed {
            changed = false;
            for constraint in &self.constraints {
                let sub = &self.values[constraint.sub.index].clone();
                let sup = &mut self.values[constraint.sup.index];
                log!("constraint: {:?}", constraint);
                log!("    sub (before): {:?}", sub);
                log!("    sup (before): {:?}", sup);
                changed |= dfs.copy(sub, sup, constraint.point);
                log!("    sup (after) : {:?}", sup);
                log!("    changed     : {:?}", changed);
            }
        }
    }
}

struct Dfs<'env> {
    stack: Vec<Point>,
    visited: HashSet<Point>,
    env: &'env Environment<'env>,
}

impl<'env> Dfs<'env> {
    fn new(env: &'env Environment<'env>) -> Self {
        Dfs {
            stack: vec![],
            visited: HashSet::new(),
            env,
        }
    }

    fn copy(&mut self, from_region: &Region, to_region: &mut Region, start_point: Point) -> bool {
        let mut changed = false;

        self.stack.clear();
        self.visited.clear();

        self.stack.push(start_point);
        while let Some(p) = self.stack.pop() {
            log!("        dfs: p={:?}", p);

            if !from_region.may_contain(p) {
                log!("            not in from-region");
                continue;
            }

            if !self.visited.insert(p) {
                log!("            already visited");
                continue;
            }

            changed |= to_region.add_point(p);

            let successor_points = self.env.successor_points(p);
            if successor_points.is_empty() {
                // If we reach the END point in the graph, then copy
                // over any skolemized end points in the `from_region`
                // and make sure they are included in the `to_region`.
                for region_decl in self.env.graph.free_regions() {
                    let block = self.env.graph.skolemized_end(region_decl.name);
                    let skolemized_end_point = Point { block, action: 0 };
                    changed |= to_region.add_point(skolemized_end_point);
                }
            } else {
                self.stack.extend(successor_points);
            }
        }

        changed
    }
}
