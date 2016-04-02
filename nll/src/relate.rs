use env::*;
use nll_repr::repr::*;

impl<'func, 'arena> Environment<'func, 'arena> {
    pub fn relate_types(&self,
                        variance: Variance,
                        ty1: Ty<'arena>,
                        ty2: Ty<'arena>)
                        -> EnvResult<'arena, Vec<RegionRelation<'arena>>>
    {
        match (&ty1.data, &ty2.data) {
            (&TyData::Usize, &TyData::Usize) => Ok(vec![]),

            (&TyData::StructRef(ref s1), &TyData::StructRef(ref s2)) => {
                if s1.name != s2.name {
                    Err(Error::Types(ty1, ty2))?;
                }

                let data = self.struct_data(s1.name)?;
                if s1.substitutions.len() != data.variances.len() {
                    Err(Error::WrongNumberArg(s1.name,
                                              data.variances.len(),
                                              s1.substitutions.len()))?;
                }
                if s2.substitutions.len() != data.variances.len() {
                    Err(Error::WrongNumberArg(s2.name,
                                              data.variances.len(),
                                              s2.substitutions.len()))?;
                }
            }
        }
    }
}
