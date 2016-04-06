use arena;
use intern::InternedString;
use lalrpop_util::ParseError;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::Hash;

mod parser;

pub struct Ballast {
    pub regions: arena::Arena<RegionData>,
}

impl Ballast {
    pub fn new() -> Self {
        Ballast {
            regions: arena::Arena::new(),
        }
    }
}

pub struct Arena<'arena> {
    pub ballast: &'arena Ballast,
    pub regions_map: HashMap<RegionData, Region<'arena>>,
}

impl<'arena> Arena<'arena> {
    pub fn new(ballast: &'arena Ballast) -> Self {
        Arena {
            ballast: ballast,
            regions_map: HashMap::default(),
        }
    }

    pub fn intern<I: 'arena + Intern<'arena>>(&mut self, data: I) -> I::Interned {
        let (arena, map) = I::fields(self);
        map.entry(data.clone())
           .or_insert_with(|| I::make(arena.alloc(data)))
           .clone()
    }
}

pub trait Intern<'arena>: Sized + Clone + Hash + Eq {
    type Interned: Clone;

    fn fields<'r>(arena: &'r mut Arena<'arena>)
              -> (&'arena arena::Arena<Self>,
                  &'r mut HashMap<Self, Self::Interned>);

    fn make(data: &'arena Self) -> Self::Interned;
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func<'arena> {
    pub data: BTreeMap<BasicBlock, BasicBlockData<'arena>>,
    pub assertions: Vec<Assertion<'arena>>
}

impl<'arena> Func<'arena> {
    pub fn parse(arena: &mut Arena<'arena>, s: &str) -> Result<Self, String> {
        let err_loc = match parser::parse_Func(arena, s) {
            Ok(f) => return Ok(f),
            Err(ParseError::InvalidToken { location }) => location,
            Err(ParseError::UnrecognizedToken { token: None, .. }) => s.len(),
            Err(ParseError::UnrecognizedToken { token: Some((l, _, _)), .. }) => l,
            Err(ParseError::ExtraToken { token: (l, _, _) }) => l,
            Err(ParseError::User { .. }) => unimplemented!()
        };

        let line_num = s[..err_loc].lines().count();
        let col_num = s[..err_loc].lines().last().map(|s| s.len()).unwrap_or(0);
        Err(format!("parse error at {}:{} (offset {})", line_num, col_num + 1, err_loc))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlockData<'arena> {
    pub name: BasicBlock,
    pub actions: Vec<Action<'arena>>,
    pub successors: Vec<BasicBlock>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Assertion<'arena> {
    RegionEq(Region<'arena>, Region<'arena>),
    RegionContains(Region<'arena>, RegionExit),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action<'arena> {
    Subregion(Region<'arena>, Region<'arena>),
    Eqregion(Region<'arena>, Region<'arena>),
    Deref(RegionVariable),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionVariable(pub InternedString);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Region<'arena> {
    pub data: &'arena RegionData
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RegionData {
    Variable(RegionVariable),
    Exits(Vec<RegionExit>),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum RegionExit {
    Point(BasicBlock, usize),
}

impl<'arena> Intern<'arena> for RegionData {
    type Interned = Region<'arena>;

    fn fields<'r>(arena: &'r mut Arena<'arena>)
                  -> (&'arena arena::Arena<Self>,
                      &'r mut HashMap<Self, Self::Interned>)
    {
        (&arena.ballast.regions, &mut arena.regions_map)
    }

    fn make(data: &'arena Self) -> Self::Interned {
        Region { data: data }
    }
}
