use intern::InternedString;
use lalrpop_util::ParseError;
use std::collections::BTreeMap;

mod parser;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func {
    pub decls: Vec<VarDecl>,
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
pub struct VarDecl {
    pub name: Variable,
    pub ty: Ty<()>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ty<R> {
    pub name: InternedString,
    pub args: Vec<TyArg<R>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum TyArg<R> {
    Region(R),
    Ty(Ty<R>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlockData {
    pub name: BasicBlock,
    pub actions: Vec<Action>,
    pub successors: Vec<BasicBlock>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    Borrow(Variable, RegionName), // p = &'X;
    Assign(Variable, Variable), // p = q;
    Use(Variable), // use(p);
    Noop,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Variable {
    name: InternedString
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct RegionName {
    name: InternedString
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Assertion {
    pub name: RegionName,
    pub region: Region,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Region {
    pub parts: Vec<RegionPart>
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RegionPart {
    pub block: BasicBlock,
    pub start: usize,
    pub end: usize,
}

