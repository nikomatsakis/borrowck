use env::{Environment, Point};
use graph::BasicBlockIndex;
use graph_algorithms::Graph;
use nll_repr::repr;
use std::error::Error;
use region::Region;
use region_map::RegionMap;
use type_map::{Assignments, TypeMap};

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let mut type_map = TypeMap::new(&env.graph);
    let mut region_map = RegionMap::new();

    // Step 1. Visit blocks and, for each block, create a unique copy
    // of the types.
    for &block in &env.reverse_post_order {
        let assignments = type_map.assignments_mut(block);
        populate_entry(&env.graph.decls(), &mut assignments.entry, &mut region_map);

        // Walk through the actions, updating the type assignment as
        // we go. Create intra-block constraints and assertions.
        assignments.exit = walk_actions(env.graph.decls(),
                                        &assignments.entry,
                                        block,
                                        &env.graph.block_data(block).actions,
                                        &mut region_map);
    }

    // Step 2. Visit blocks and create inter-block subtyping constraints.
    //
    // If B1 -> B2, then B1.exit <: B2.entry.
    for &pred in &env.reverse_post_order {
        let pred_assignments = &type_map.assignments(pred).exit;
        let pred_actions = env.graph.block_data(pred).actions.len();
        let pred_end = Point { block: pred, action: pred_actions };
        for succ in env.graph.successors(pred) {
            let succ_assignments = &type_map.assignments(succ).entry;
            let succ_start = Point { block: succ, action: 0 };
            for &var in env.graph.decls() {
                let pred_ty = pred_assignments.get(var);
                let succ_ty = succ_assignments.get(var);
                region_map.flow(pred_ty, pred_end, succ_start);
                region_map.subtype(pred_ty, succ_ty); // pred_ty <: succ_ty
            }
        }
    }

    // Step 3. Convert and register the user assertions.
    for assertion in env.graph.assertions() {
        match *assertion {
            repr::Assertion::Eq(name, ref region) => {
                let r = to_region(env, region);
                region_map.assert_region_eq(name, r);
            }

            repr::Assertion::In(name, ref point) => {
                let p = to_point(env, point);
                region_map.assert_region_contains(name, p, true);
            }

            repr::Assertion::NotIn(name, ref point) => {
                let p = to_point(env, point);
                region_map.assert_region_contains(name, p, false);
            }
        }
    }

    // Step 4. Find solutions.
    let solution = region_map.solve();

    // Step 5. Check assertions.
    let errors = solution.check();

    if errors > 0 {
        try!(Err(format!("{} errors found", errors)));
    }

    Ok(())
}

fn populate_entry(decls: &[repr::Variable],
                  assignment: &mut Assignments,
                  region_map: &mut RegionMap)
{
    for &decl in decls {
        let ty = region_map.fresh_ty();
        assignment.set_var(decl, ty);
    }
}

fn walk_actions(decls: &[repr::Variable],
                assignment_on_entry: &Assignments,
                block: BasicBlockIndex,
                actions: &[repr::Action],
                region_map: &mut RegionMap)
                -> Assignments
{
    let mut assignments = assignment_on_entry.clone();
    for (index, action) in actions.iter().enumerate() {
        let current_point = Point { block: block, action: index };
        match *action {
            // `p = &` -- create a new type for `p`, since it is being
            // overridden. The old type is dead so it need not contain
            // this point.
            repr::Action::Borrow(var, read_name, write_name) => {
                let new_ty = region_map.fresh_ty();
                assignments.set_var(var, new_ty);
                region_map.read_ref(new_ty, current_point);
                region_map.user_names(read_name, write_name, new_ty);
            }

            // a = b
            repr::Action::Assign(a, b) => {
                let a_ty = region_map.fresh_ty();
                assignments.set_var(a, a_ty);
                region_map.read_ref(a_ty, current_point);

                let b_ty = assignments.get(b);
                region_map.subtype(b_ty, a_ty);
            }

            repr::Action::Use(var) => {
                let var_ty = assignments.get(var);
                region_map.read_ref(var_ty, current_point);
            }

            repr::Action::Noop => {
            }
        }

        let next_point = Point { block: block, action: index + 1 };
        for &var in decls {
            region_map.flow(assignments.get(var), current_point, next_point);
        }
    }

    assignments
}

fn to_region(env: &Environment, region: &repr::Region) -> Region {
    let mut result = Region::new();
    for part in &region.parts {
        let block = env.graph.block(part.block);
        if part.start != part.end {
            let start_point = Point { block: block, action: part.start };
            let end_point = Point { block: block, action: part.end - 1 };
            result.add_point(start_point);
            result.add_point(end_point);
        }
    }
    result
}

fn to_point(env: &Environment, point: &repr::Point) -> Point {
    let block = env.graph.block(point.block);
    Point { block: block, action: point.action }
}
