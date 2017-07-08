use nll_repr::repr;
use std::iter::once;

pub trait DefUse {
    /// Returns path that this action overwrites, if any.
    fn overwrites(&self) -> Option<&repr::Path>;

    /// Returns (defs, uses), where `defs` contains variables whose
    /// current value is completely overwritten, and `uses` contains
    /// variables whose current value is used. Note that a variable
    /// may exist in both sets.
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>);
}

impl DefUse for repr::Action {
    fn overwrites(&self) -> Option<&repr::Path> {
        match *self {
            repr::Action::Borrow(ref p, _name, _, _) => Some(p),
            repr::Action::Init(ref a, _) => Some(a),
            repr::Action::Assign(ref a, _) => Some(a),
            repr::Action::Constraint(ref _c) => None,
            repr::Action::Use(_) => None,
            repr::Action::Write(_) => None, // ???
            repr::Action::Drop(_) => None,
            repr::Action::Noop => None,
        }
    }

    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>) {
        match *self {
            repr::Action::Borrow(ref p, _name, _, ref q) => (vec![p.base()], vec![q.base()]),
            repr::Action::Init(ref a, ref params) => {
                (a.write_def().into_iter().collect(),
                 params.iter().map(|p| p.base()).chain(a.write_use()).collect())
            }
            repr::Action::Assign(ref a, ref b) => {
                (a.write_def().into_iter().collect(),
                 once(b.base()).chain(a.write_use()).collect())
            }
            repr::Action::Constraint(ref _c) => (vec!(), vec!()),
            repr::Action::Use(ref v) => (vec!(), vec!(v.base())),
            repr::Action::Write(ref v) => (vec!(), vec!(v.base())),

            // drop is special
            repr::Action::Drop(..) => (vec!(), vec!()),

            repr::Action::Noop => (vec!(), vec!()),
        }
    }
}
