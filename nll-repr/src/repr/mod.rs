use intern::{self, InternedString};
use lalrpop_util::ParseError;
use std::collections::BTreeMap;
use std::iter;

mod parser;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func {
    pub decls: Vec<VariableDecl>,
    pub structs: Vec<StructDecl>,
    pub data: BTreeMap<BasicBlock, BasicBlockData>,
    pub assertions: Vec<Assertion>
}

impl Func {
    pub fn parse(s: &str) -> Result<Self, String> {
        let err_loc = match parser::parse_Func(s) {
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
pub struct StructDecl {
    pub name: StructName,
    pub parameters: Vec<StructParameter>,
    pub fields: Vec<FieldDecl>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: FieldName,
    pub ty: Box<Ty>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructParameter {
    pub kind: Kind,
    pub variance: Variance,
    pub may_dangle: bool,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Kind {
    Region,
    Type,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Variance {
    Co,
    Contra,
    In,
}

impl Variance {
    pub fn invert(self) -> Variance {
        match self {
            Variance::Co => Variance::Contra,
            Variance::Contra => Variance::Co,
            Variance::In => Variance::In,
        }
    }

    pub fn xform(self, v: Variance) -> Variance {
        match self {
            Variance::Co => v,
            Variance::Contra => v.invert(),
            Variance::In => Variance::In,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructName {
    name: InternedString
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Ty {
    Ref(Region, BorrowKind, Box<Ty>),
    Unit,
    Struct(StructName, Vec<TyParameter>),
    Bound(usize),
}

impl Ty {
    pub fn subst(&self, params: &[TyParameter]) -> Ty {
        match *self {
            Ty::Bound(b) => {
                let index = params.len() - 1 - b;
                match params[index] {
                    TyParameter::Ty(ref t) => (**t).clone(),
                    TyParameter::Region(r) => {
                        panic!("subst: encountered region {:?} at index {} not type", r, index)
                    }
                }
            }
            Ty::Ref(rn, kind, ref t) => Ty::Ref(rn.subst(params), kind, Box::new(t.subst(params))),
            Ty::Unit => Ty::Unit,
            Ty::Struct(s, ref params) => Ty::Struct(
                s,
                params.iter().map(|p| p.subst(params)).collect()
            ),
        }
    }

    pub fn walk_regions<'a>(&'a self) -> Box<Iterator<Item = Region> + 'a> {
        match *self {
            Ty::Ref(rn, _kind, ref t) => Box::new(
                iter::once(rn).chain(t.walk_regions())
            ),
            Ty::Unit => Box::new(
                iter::empty()
            ),
            Ty::Struct(_, ref params) => Box::new(
                params.iter()
                      .flat_map(move |p| match *p {
                          TyParameter::Region(rn) => Box::new(iter::once(rn)),
                          TyParameter::Ty(ref t) => t.walk_regions(),
                      })
            ),
            Ty::Bound(_) => {
                panic!("encountered bound type when walking regions")
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Region {
    Free(RegionName),
    Bound(usize),
}

impl Region {
    pub fn subst(self, params: &[TyParameter]) -> Region {
        match self {
            Region::Free(..) => self,
            Region::Bound(b) => {
                let index = params.len() - 1 - b;
                match params[index] {
                    TyParameter::Region(r) => r,
                    TyParameter::Ty(ref t) => {
                        panic!("subst: encountered type {:?} at index {} not region", t, index)
                    }
                }
            }
        }
    }

    pub fn assert_free(self) -> RegionName {
        match self {
            Region::Free(n) => n,
            Region::Bound(b) => panic!("assert_free: encountered bound region with depth {}", b),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum TyParameter {
    Region(Region),
    Ty(Box<Ty>),
}

impl TyParameter {
    pub fn subst(&self, params: &[TyParameter]) -> TyParameter {
        match *self {
            TyParameter::Region(r) => TyParameter::Region(r.subst(params)),
            TyParameter::Ty(ref t) => TyParameter::Ty(Box::new(t.subst(params))),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlockData {
    pub name: BasicBlock,
    pub actions: Vec<Action>,
    pub successors: Vec<BasicBlock>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum BorrowKind {
    Mut,
    Shared,
}

impl BorrowKind {
    pub fn variance(self) -> Variance {
        match self {
            BorrowKind::Mut => Variance::In,
            BorrowKind::Shared => Variance::Co,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    Init(Box<Path>, Vec<Box<Path>>), // p = use(...)
    Borrow(Box<Path>, RegionName, BorrowKind, Box<Path>), // p = &'X q
    Assign(Box<Path>, Box<Path>), // p = q;
    Constraint(Box<Constraint>), // C
    Use(Box<Path>), // use(p);
    Drop(Box<Path>), // drop(p);
    Noop,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Path { // P =
    Base(Variable), // v
    Extension(Box<Path>, FieldName), // P.n
}

impl Path {
    pub fn base(&self) -> Variable {
        match *self {
            Path::Base(v) => v,
            Path::Extension(ref e, _) => e.base(),
        }
    }

    /// When you have `p = ...`, which variable is reassigned?
    /// If this is `p = x`, then `x` is. Otherwise, nothing.
    pub fn write_def(&self) -> Option<Variable> {
        match *self {
            Path::Base(v) => Some(v),
            Path::Extension(..) => None,
        }
    }

    /// When you have `p = ...`, which variable is read?
    /// If this is `p = x.0`, then `x` is. Otherwise, nothing.
    pub fn write_use(&self) -> Option<Variable> {
        match *self {
            Path::Base(..) => None,
            Path::Extension(..) => Some(self.base()),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Constraint {
    ForAll(Vec<RegionName>, Box<Constraint>),
    Exists(Vec<RegionName>, Box<Constraint>),
    Implies(Vec<OutlivesConstraint>, Box<Constraint>),
    All(Vec<Constraint>),
    Outlives(OutlivesConstraint),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct OutlivesConstraint {
    pub sup: RegionName,
    pub sub: RegionName,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Variable {
    name: InternedString,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct VariableDecl {
    pub var: Variable,
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Assertion {
    Eq(RegionName, RegionLiteral),
    In(RegionName, Point),
    NotIn(RegionName, Point),
    Live(Variable, BasicBlock),
    NotLive(Variable, BasicBlock),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Point {
    pub block: BasicBlock,
    pub action: usize,
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct RegionName {
    name: InternedString
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct FieldName {
    name: InternedString
}

impl FieldName {
    pub fn star() -> Self {
        FieldName { name: intern::intern("star") }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RegionLiteral {
    pub points: Vec<Point>,
}
