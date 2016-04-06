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
    for &block in &env.reverse_post_order {
        let actions = &env.graph.block_data(block).actions;
        for (index, action) in actions.iter().enumerate() {
            match *action {
                repr::Action::Subregion(..) => unimplemented!(),
                repr::Action::Eqregion(..) => unimplemented!(),
                repr::Action::Deref(name) => {
                    grow(env, &mut region_map, name, block, index);
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
            let exits = exits.iter()
                             .map(|exit| {
                                 match *exit {
                                     repr::RegionExit::Point(block, index) => {
                                         (env.graph.block_index(block), index)
                                     }
                                 }
                             })
                             .collect();
            Region::new(exits)
        }
    }
}

fn grow(env: &Environment,
        region_map: &mut HashMap<repr::RegionVariable, Region>,
        name: repr::RegionVariable,
        block: BasicBlockIndex,
        action: usize) {
    let point = Point {
        block: block,
        action: action + 1,
    };

    if let Some(region) = region_map.get_mut(&name) {
        region.add_point(env, point);
        return;
    }

    region_map.insert(name, Region::with_point(point));
}
