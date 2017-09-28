use env::{Environment, Point};
use nll_repr::repr;
use region::Region;
use std::collections::HashSet;
use std::mem;

pub struct InferenceContext {
    /// for each region variable, sets of points where live data in
    /// the region exists
    definitions: Vec<VarDefinition>,
    constraints: Vec<Constraint>,

    /// `solve()`, `add_live_point()` and other such routines can grow
    /// this vector. It is returned by the call to `solve()`.
    errors: Vec<InferenceError>,
}

/// Inference errors occur when the constraints would force us to
/// grow a "locked region".
pub struct InferenceError {
    /// Due to a constraint at this point...
    pub constraint_point: Point,

    /// ...this capped region exceeded its cap.
    pub name: repr::RegionName,
}

/// For each inference variable that has been allocated, we have one
/// of these structures. Inference variables are "named" by their
/// index in the main vector, using an instance of `RegionVariable`.
struct VarDefinition {
    name: repr::RegionName,

    /// The current value of this inference variable. This is adjusted
    /// during regionck by calls to `add_live_point`, and then finally
    /// adjusted further by the call to `solve()`.
    value: Region,

    /// "Capped" inference variables should no longer have to grow as
    /// a result of inference. If they *do* wind up growing, we will
    /// report an error.
    capped: bool,
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
            definitions: vec![],
            constraints: vec![],
            errors: vec![],
        }
    }

    pub fn add_var(&mut self, name: repr::RegionName) -> RegionVariable {
        let index = self.definitions.len();
        self.definitions.push(VarDefinition {
            name,
            value: Region::new(),
            capped: false,
        });
        RegionVariable { index }
    }

    pub fn cap_var(&mut self, v: RegionVariable) {
        self.definitions[v.index].capped = true;
    }

    pub fn add_live_point(&mut self, v: RegionVariable, point: Point) {
        log!("add_live_point({:?}, {:?})", v, point);
        let definition = &mut self.definitions[v.index];
        if definition.value.add_point(point) {
            if definition.capped {
                self.errors.push(InferenceError {
                    constraint_point: point,
                    name: definition.name,
                });
            }
        }
    }

    pub fn add_outlives(&mut self, sup: RegionVariable, sub: RegionVariable, point: Point) {
        log!("add_outlives({:?}: {:?} @ {:?})", sup, sub, point);
        self.constraints.push(Constraint { sup, sub, point });
    }

    pub fn region(&self, v: RegionVariable) -> &Region {
        &self.definitions[v.index].value
    }

    pub fn solve(&mut self, env: &Environment) -> Vec<InferenceError> {
        let mut changed = true;
        let mut dfs = Dfs::new(env);
        while changed {
            changed = false;
            for constraint in &self.constraints {
                let sub = &self.definitions[constraint.sub.index].value.clone();
                let sup_def = &mut self.definitions[constraint.sup.index];
                log!("constraint: {:?}", constraint);
                log!("    sub (before): {:?}", sub);
                log!("    sup (before): {:?}", sup_def.value);

                if dfs.copy(sub, &mut sup_def.value, constraint.point) {
                    changed = true;

                    if sup_def.capped {
                        // This is kind of a hack, but when we add a
                        // constraint, the "point" is always the point
                        // AFTER the action that induced the
                        // constraint. So report the error on the
                        // action BEFORE that.
                        assert!(constraint.point.action > 0);
                        let p = Point { block: constraint.point.block,
                                        action: constraint.point.action - 1 };

                        self.errors.push(InferenceError {
                            constraint_point: p,
                            name: sup_def.name,
                        });
                    }
                }

                log!("    sup (after) : {:?}", sup_def.value);
                log!("    changed     : {:?}", changed);
            }
            log!("\n");
        }

        mem::replace(&mut self.errors, vec![])
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
