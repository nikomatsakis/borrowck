use env::{Environment, Point};
use graph::BasicBlockIndex;
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
            println!("Actions from {:?}/{:?}", block, env.graph.block_name(block));
            let actions = &env.graph.block_data(block).actions;
            for (index, action) in actions.iter().enumerate() {
                match *action {
                    repr::Action::Subregion(sub, sup) => {
                        if subregion(env, &mut region_map, sub, sup) {
                            println!("changed!");
                            changed = true;
                        }
                    }
                    repr::Action::Deref(name) => {
                        if grow(env, &mut region_map, name, block, index) {
                            println!("changed!");
                            changed = true;
                        }
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
                    if errors > 0 { println!(""); }
                    println!("Region {:?} is {:#?}", r1, rr1);
                    println!("Region {:?} is {:#?}", r2, rr2);
                    println!("But they should be equal!");
                    errors += 1;
                }
            }

            repr::Assertion::RegionContains(r, ref p) => {
                let rr = lookup(env, &region_map, r);
                let pp = point(env, p);
                if !rr.contains(env, pp) {
                    if errors > 0 { println!(""); }
                    println!("Region {:?} is {:#?}", r, rr);
                    println!("Point {:?} is {:#?}", p, pp);
                    println!("Region does not contain point.");
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

        repr::RegionData::Exits(ref exits) => {
            let exits = exits.iter().map(|exit| point(env, exit));
            Region::with_exits(exits)
        }
    }
}

fn point<'func, 'arena>(env: &Environment<'func, 'arena>,
                        point: &repr::RegionExit)
                        -> Point {
    match *point {
        repr::RegionExit::Point(block, index) => {
            Point { block: env.graph.block_index(block), action: index }
        }
    }
}

fn grow(env: &Environment,
        region_map: &mut HashMap<repr::RegionVariable, Region>,
        name: repr::RegionVariable,
        block: BasicBlockIndex,
        action: usize)
        -> bool {
    let point = Point {
        block: block,
        action: action,
    };

    if let Some(region) = region_map.get_mut(&name) {
        if !region.contains(env, point) {
            region.add_point(env, point);
            return true;
        } else {
            println!("region {:?} contains {:?}", region, point);
            return false;
        }
    }

    region_map.insert(name, Region::with_point(point));
    true
}

fn subregion<'func, 'arena>(env: &Environment<'func, 'arena>,
                            region_map: &mut HashMap<repr::RegionVariable, Region>,
                            sub: repr::Region<'arena>,
                            sup: repr::Region<'arena>)
                            -> bool {
    let sup_name = match *sup.data {
        repr::RegionData::Variable(name) => name,
        repr::RegionData::Exits(_) => return false
    };

    let in_region = lookup(env, region_map, sub);
    for exit in in_region.exits() {
    }

    unimplemented!()
}
