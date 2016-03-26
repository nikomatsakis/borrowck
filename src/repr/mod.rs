use arena;
use intern::InternedString;
use lalrpop_util::ParseError;
use std::collections::HashMap;
use std::hash::Hash;

mod parser;
#[cfg(test)] mod test;

pub struct Ballast<'arena> {
    pub types: arena::Arena<TyData<'arena>>,
    pub regions: arena::Arena<RegionData>,
}

impl<'arena> Ballast<'arena> {
    pub fn new() -> Self {
        Ballast {
            types: arena::Arena::new(),
            regions: arena::Arena::new(),
        }
    }
}

pub struct Arena<'arena> {
    pub ballast: &'arena Ballast<'arena>,
    pub types_map: HashMap<TyData<'arena>, Ty<'arena>>,
    pub regions_map: HashMap<RegionData, Region<'arena>>,
}

impl<'arena> Arena<'arena> {
    pub fn new(ballast: &'arena Ballast<'arena>) -> Self {
        Arena {
            ballast: ballast,
            types_map: HashMap::default(),
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

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func<'arena> {
    data: HashMap<BasicBlock, BasicBlockData<'arena>>
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
    name: BasicBlock,
    actions: Vec<Action<'arena>>,
    successors: Vec<BasicBlock>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action<'arena> {
    Subtype(Ty<'arena>, Ty<'arena>),
    Deref(Ty<'arena>)
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ty<'arena> {
    pub data: &'arena TyData<'arena>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum TyData<'arena> {
    Usize,
    Structure(Structure<'arena>),
    Reference(Reference<'arena>),
    Parameter(InternedString),
}

impl<'arena> Intern<'arena> for TyData<'arena> {
    type Interned = Ty<'arena>;

    fn fields<'r>(arena: &'r mut Arena<'arena>)
              -> (&'arena arena::Arena<Self>,
                  &'r mut HashMap<Self, Self::Interned>)
    {
        (&arena.ballast.types, &mut arena.types_map)
    }

    fn make(data: &'arena Self) -> Self::Interned {
        Ty { data: data }
    }
}

/// an instance of some kind K = Ty | Region
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Atom<'arena> {
    Type(Ty<'arena>),
    Region(Region<'arena>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Structure<'arena> {
    name: InternedString,
    substitutions: Vec<Atom<'arena>>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Reference<'arena> {
    region: Region<'arena>,
    mutability: Mutability,
    ty: Ty<'arena>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Mutability {
    Mut,
    NotMut,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Region<'arena> {
    pub data: &'arena RegionData
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum RegionData {
    Parameter(InternedString),
    Variable(InternedString),
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
