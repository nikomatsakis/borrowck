use intern::InternedString;
use lalrpop_util::ParseError;
use std::collections::BTreeMap;

mod parser;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BasicBlock(pub InternedString);

#[derive(Clone, Debug)]
pub struct Func {
    pub decls: Vec<VariableDecl>,
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
pub struct BasicBlockData {
    pub name: BasicBlock,
    pub actions: Vec<Action>,
    pub successors: Vec<BasicBlock>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    Borrow(Variable, RegionName), // p = &'X
    Assign(Variable, Variable), // p = q;
    Subregion(RegionName, RegionName), // 'x <= 'y;
    Use(Variable), // use(p);
    Write(Variable), // write(p);
    Noop,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Variable {
    name: InternedString,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct VariableDecl {
    pub var: Variable,
    pub region: RegionName,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Assertion {
    Eq(RegionName, Region),
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

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct RegionName {
    name: InternedString
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Region {
    pub points: Vec<Point>,
}
