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

        for var_decl in env.graph.decls() {
            region_map.enter_ty(&assignments.entry.get(var_decl.name), block);
        }

        // Walk through the actions, updating the type assignment as
        // we go. Create intra-block constraints and assertions.
        assignments.exit = walk_actions(&assignments.entry,
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
            for var in env.graph.decls().iter().map(|d| d.name) {
                let pred_ty = pred_assignments.get(var);
                let succ_ty = succ_assignments.get(var);
                region_map.goto(pred_ty, pred_end, succ_ty, succ_start);
            }
        }
    }

    // Step 3. Convert and register the user assertions.
    for assertion in env.graph.assertions() {
        let r = to_region(env, &assertion.region);
        region_map.assert_region(assertion.name, r);
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

fn populate_entry(decls: &[repr::VarDecl],
                  assignment: &mut Assignments,
                  region_map: &mut RegionMap)
{
    for decl in decls {
        let ty = region_map.instantiate_ty(&decl.ty);
        assignment.set_var(decl.name, ty);
    }
}

fn walk_actions(assignment_on_entry: &Assignments,
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
            repr::Action::Borrow(var, region_name) => {
                let new_ty = {
                    let old_ty = assignments.get(var);
                    region_map.instantiate_ty(old_ty)
                };
                assignments.set_var(var, new_ty);
                let new_ty = assignments.get(var);
                region_map.use_ty(&new_ty, current_point);
                region_map.user_names(region_name, new_ty);
            }

            // a = b
            repr::Action::Assign(a, b) => {
                let a_ty = {
                    let old_a_ty = assignments.get(a);
                    region_map.instantiate_ty(old_a_ty)
                };
                assignments.set_var(a, a_ty);

                let a_ty = assignments.get(a);
                let b_ty = assignments.get(b);
                let next_point = Point { block, action: index + 1 };
                region_map.goto(b_ty, current_point, a_ty, next_point);
            }

            repr::Action::Use(var) => {
                let var_ty = assignments.get(var);
                region_map.use_ty(var_ty, current_point);
            }

            repr::Action::Noop => {
            }
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
