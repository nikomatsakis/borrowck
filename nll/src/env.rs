use graph::FuncGraph;
use nll_repr::repr::*;

pub struct Environment<'func, 'arena: 'func> {
    pub graph: &'func FuncGraph<'arena>,
}

impl<'func, 'arena> Environment<'func, 'arena> {
    pub fn struct_data(&self, name: StructName)
                       -> EnvResult<'arena, &'func StructData> {
        match self.graph.func().structs.get(&name) {
            Some(v) => Ok(v),
            None => Err(Error::NoStructData(name))
        }
    }
}

pub struct RegionRelation<'arena> {
    variance: Variance,
    r1: Region<'arena>,
    r2: Region<'arena>,
}

pub enum Error<'arena> {
    Types(Ty<'arena>, Ty<'arena>),
    NoStructData(StructName),
    WrongNumberArg(StructName, usize, usize),
}

pub type EnvResult<'arena, T> = Result<T, Error<'arena>>;

pub trait VarianceMethods {
    fn invert(self) -> Variance;
    fn xform(self, context: Variance) -> Variance;
}

impl VarianceMethods for Variance {
    fn invert(self) -> Variance {
        match self {
            Variance::Co => Variance::Contra,
            Variance::Contra => Variance::Co,
            Variance::In => Variance::In,
        }
    }

    fn xform(self, context: Variance) -> Variance {
        match context {
            Variance::Co => self,
            Variance::Contra => self.invert(),
            Variance::In => Variance::In,
        }
    }
}
