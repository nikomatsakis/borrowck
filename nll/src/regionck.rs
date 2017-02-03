use env::{Environment, Point};
use region::Region;
use std::collections::HashMap;
use std::error::Error;
use nll_repr::repr;

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let mut region_map = HashMap::new();

    // Visit the blocks in reverse post order, for no particular
    // reason, just because it's convenient.
    let mut changed = true;
    while changed {
        changed = false;
        for &block in &env.reverse_post_order {
            let actions = &env.graph.block_data(block).actions;
            for (index, action) in actions.iter().enumerate() {
                log!("Action {:?} from {:?} is {:?}",
                     index, block, action);
                let current_point = Point {
                    block: block,
                    action: index,
                };
                match *action {
                    repr::Action::Subregion(sub, sup) => {
                        if subregion(env, &mut region_map, sub, sup) {
                            log!("changed!");
                            changed = true;
                        }
                    }
                    repr::Action::Deref(name) => {
                        if grow(&mut region_map, name, current_point) {
                            log!("changed!");
                            changed = true;
                        }
                    }
                    repr::Action::Noop => {
                    }
                }
            }
        }
    }

    let mut errors = 0;
    for assertion in env.graph.assertions() {
        match *assertion {
            repr::Assertion::RegionEq(r1, r2) => {
                let rr1 = lookup(env, &region_map, r1);
                let rr2 = lookup(env, &region_map, r2);
                if rr1 != rr2 {
                    if errors > 0 {
                        println!("");
                    }
                    println!("Region {:?} is {:#?}", r1, rr1);
                    println!("Region {:?} is {:#?}", r2, rr2);
                    println!("Regions are not equal (but they should be).");
                    errors += 1;
                }
            }

            repr::Assertion::RegionContains(r, ref p) => {
                let rr = lookup(env, &region_map, r);
                let pp = point(env, p);
                if !rr.contains(pp) {
                    if errors > 0 {
                        println!("");
                    }
                    println!("Region {:?} is {:#?}", r, rr);
                    println!("Point {:?} is {:#?}", p, pp);
                    println!("Region does not contain point (but should).");
                    errors += 1;
                }
            }

            repr::Assertion::RegionNotContains(r, ref p) => {
                let rr = lookup(env, &region_map, r);
                let pp = point(env, p);
                if rr.contains(pp) {
                    if errors > 0 {
                        println!("");
                    }
                    println!("Region {:?} is {:#?}", r, rr);
                    println!("Point {:?} is {:#?}", p, pp);
                    println!("Region contains point (but should not).");
                    errors += 1;
                }
            }
        }
    }

    if errors > 0 {
        try!(Err(format!("{} errors found", errors)));
    }

    Ok(())
}

fn lookup<'func, 'arena>(env: &Environment<'func, 'arena>,
                         region_map: &HashMap<repr::RegionVariable, Region>,
                         region: repr::Region<'arena>)
                         -> Region {
    match *region.data {
        repr::RegionData::Variable(name) => region_map[&name].clone(),

        repr::RegionData::Literal(ref range) => {
            let mut region = Region::new();
            for r in range {
                let block = env.graph.block_index(r.block);
                if r.end_action > r.start_action {
                    region.add_point(Point { block: block, action: r.start_action });
                    region.add_point(Point { block: block, action: r.end_action - 1 });
                }
            }
            region
        }
    }
}

fn point<'func, 'arena>(env: &Environment<'func, 'arena>, point: &repr::Point) -> Point {
    Point {
        block: env.graph.block_index(point.block),
        action: point.action,
    }
}

fn grow(region_map: &mut HashMap<repr::RegionVariable, Region>,
        name: repr::RegionVariable,
        point: Point)
        -> bool {
    region_map.entry(name)
              .or_insert(Region::new())
              .add_point(point)
}

fn subregion<'func, 'arena>(env: &Environment<'func, 'arena>,
                            region_map: &mut HashMap<repr::RegionVariable, Region>,
                            sub: repr::Region<'arena>,
                            sup: repr::Region<'arena>)
                            -> bool {
    let sub_region = lookup(env, region_map, sub);

    let sup_name = match *sup.data {
        repr::RegionData::Variable(name) => name,
        repr::RegionData::Literal(..) => return false,
    };

    region_map.entry(sup_name)
              .or_insert(Region::new())
              .add_region(&sub_region)
}
