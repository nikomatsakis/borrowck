use env::{Environment, Point};
use graph::BasicBlockIndex;
use graph_algorithms::Graph;
use nll_repr::repr;
use std::error::Error;
use region_map::RegionMap;

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let mut region_map = RegionMap::new();

    for &block in &env.reverse_post_order {
        let block_actions = &env.graph.block_data(block).actions;
        walk_actions(env,
                     block,
                     block_actions,
                     &mut region_map);
    }

    for assertion in env.graph.assertions() {
        match *assertion {
            repr::Assertion::In(ref r, ref p) => {
                let r_p = to_point(env, &r.point);
                let p = to_point(env, p);
                region_map.assert_region_contains(r.variable, r_p, r.index, p, true);
            }

            repr::Assertion::NotIn(ref r, ref p) => {
                let r_p = to_point(env, &r.point);
                let p = to_point(env, p);
                region_map.assert_region_contains(r.variable, r_p, r.index, p, false);
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

fn walk_actions(env: &Environment,
                block: BasicBlockIndex,
                actions: &[repr::Action],
                region_map: &mut RegionMap)
{
    let decls = env.graph.decls();
    for (index, action) in actions.iter().enumerate() {
        let current_point = Point { block: block, action: index };
        let mut no_flow = None;
        match *action {
            // `p = &` -- create a new type for `p`, since it is being
            // overridden. The old type is dead so it need not contain
            // this point.
            repr::Action::Borrow(var) => {
                no_flow = Some(var);
                region_map.read_var(var, current_point);
            }

            // a = b
            repr::Action::Assign(a, b) => {
                no_flow = Some(a);
                region_map.read_var(a, current_point);
                region_map.read_var(b, current_point);
                region_map.subregion(a, b, current_point);
            }

            // use(a); i.e., print(*a)
            repr::Action::Use(var) => {
                region_map.read_var(var, current_point);
            }

            // write(a); i.e., *a += 1
            repr::Action::Write(var) => {
                region_map.write_var(var, current_point);
            }

            repr::Action::Noop => {
            }
        }

        let mut pred_points = vec![];
        if index > 0 {
            pred_points.push(Point { block: block, action: index - 1 });
        } else {
            pred_points.extend(
                env.graph.predecessors(block)
                         .map(|p| env.end_point(p)));
        }

        for pred_point in pred_points {
            flow(region_map,
                 pred_point,
                 current_point,
                 decls.iter().filter(|&&v| Some(v) != no_flow).cloned());
        }
    }

    // Add one final set of flow edges from last action to terminator;
    // terminator never kills anything.
    if actions.len() > 0 {
        flow(region_map,
             Point { block: block, action: actions.len() - 1 },
             Point { block: block, action: actions.len() },
             decls.iter().cloned());
    } else {
        for pred_point in env.graph.predecessors(block).map(|p| env.end_point(p)) {
            flow(region_map,
                 pred_point,
                 env.end_point(block),
                 decls.iter().cloned());
        }
    }
}

fn flow<I>(region_map: &mut RegionMap,
           p: Point,
           q: Point,
           decls: I)
    where I: IntoIterator<Item = repr::Variable>
{
    for var in decls {
        region_map.flow(var, p, q);
    }
}

fn to_point(env: &Environment, point: &repr::Point) -> Point {
    let block = env.graph.block(point.block);
    Point { block: block, action: point.action }
}
