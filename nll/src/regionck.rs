use env::{Environment, Point};
use graph::BasicBlockIndex;
use lalrpop_intern::InternedString;
use region::Region;
use std::collections::HashMap;
use nll_repr::repr;

pub fn region_check(env: &Environment) {
    let mut regions = HashMap::new();

    // Visit the blocks in reverse post order, for no particular
    // reason, just because it's convenient.
    for &block in &env.reverse_post_order {
        let actions = &env.graph.block_data(block).actions;
        for (index, action) in actions.iter().enumerate() {
            match *action {
                repr::Action::Subregion(..) => unimplemented!(),
                repr::Action::Eqregion(..) => unimplemented!(),
                repr::Action::Deref(region) => {
                    match *region.data {
                        repr::RegionData::Variable(name) => {
                            grow(env, &mut regions, name, block, index);
                        }
                    }
                }
            }
        }
    }

    let mut names: Vec<_> = regions.keys().cloned().collect();
    names.sort();
    for name in names {
        println!("Region {:?} is {:#?}", name, regions[&name]);
    }
}

fn grow(env: &Environment,
        region_map: &mut HashMap<InternedString, Region>,
        name: InternedString,
        block: BasicBlockIndex,
        action: usize) {
    let point = Point { block: block, action: action + 1 };

    if let Some(region) = region_map.get_mut(&name) {
        region.add_point(env, point);
        return;
    }

    region_map.insert(name, Region::with_point(point));
}
