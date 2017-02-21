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
    index: usize
}

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
        self.values[v.index].add_point(point);
    }

    pub fn add_constraint(&mut self, sub: RegionVariable, sup: RegionVariable, point: Point) {
        self.constraints.push(Constraint { sub, sup, point });
    }

    pub fn solve(&self, env: &Environment) -> Solution {
        Solution::new(self, env)
    }
}

pub struct Solution {
    values: Vec<Region>,
}

impl Solution {
    fn new(cx: &InferenceContext, env: &Environment) -> Self {
        let mut this = Solution { values: cx.values.clone() };
        this.iterate(cx, env);
        this
    }

    fn iterate(&mut self, cx: &InferenceContext, env: &Environment) {
        let mut changed = true;
        let mut dfs = Dfs::new(env);
        while changed {
            changed = false;
            for constraint in &cx.constraints {
                let sub = &self.values[constraint.sub.index].clone();
                let sup = &mut self.values[constraint.sup.index];
                changed |= dfs.copy(sub, sup, constraint.point);
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
        Dfs { stack: vec![], visited: HashSet::new(), env }
    }

    fn copy(&mut self, from_region: &Region, to_region: &mut Region, start_point: Point)
            -> bool
    {
        let mut changed = false;

        self.stack.clear();
        self.visited.clear();

        self.stack.push(start_point);
        while let Some(p) = self.stack.pop() {
            if !from_region.contains(p) {
                continue;
            }

            if !self.visited.insert(p) {
                continue;
            }

            changed |= to_region.add_point(p);

            self.stack.extend(self.env.successor_points(p));
        }

        changed
    }
}
