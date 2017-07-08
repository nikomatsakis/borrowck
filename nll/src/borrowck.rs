use env::{Environment, Point};
use loans_in_scope::{Loan, LoansInScope};
use nll_repr::repr;
use std::error::Error;
use std::fmt;

pub fn borrow_check(env: &Environment,
                    loans_in_scope: &LoansInScope)
                    -> Result<(), Box<Error>> {
    let mut result: Result<(), Box<Error>> = Ok(());
    loans_in_scope.walk(env, |point, opt_action, loans| {
        let borrowck = BorrowCheck { env, point, loans };
        if let Some(action) = opt_action {
            if let Err(e) = borrowck.check_action(action) {
                if !action.should_have_error {
                    result = Err(e);
                }
            } else if action.should_have_error {
                result = Err(Box::new(BorrowError::no_error(point)));
            }
        }
    });

    result
}

struct BorrowCheck<'cx> {
    env: &'cx Environment<'cx>,
    point: Point,
    loans: &'cx [&'cx Loan<'cx>],
}

impl<'cx> BorrowCheck<'cx> {
    fn check_action(&self, action: &repr::Action) -> Result<(), Box<Error>> {
        match action.kind {
            repr::ActionKind::Init(ref a, ref bs) => {
                self.check_write(a)?;
                for b in bs {
                    self.check_read(b)?;
                }
            }
            repr::ActionKind::Assign(ref a, ref b) => {
                self.check_write(a)?;
                self.check_read(b)?;
            }
            repr::ActionKind::Borrow(ref a, _, repr::BorrowKind::Shared, ref b) => {
                self.check_write(a)?;
                self.check_read(b)?;
            }
            repr::ActionKind::Borrow(ref a, _, repr::BorrowKind::Mut, ref b) => {
                self.check_write(a)?;
                self.check_write(b)?;
            }
            repr::ActionKind::Constraint(_) => {
            }
            repr::ActionKind::Use(ref p) => {
                self.check_read(p)?;
            }
            repr::ActionKind::Drop(ref p) => {
                self.check_move(p)?;
            }
            repr::ActionKind::StorageDead(p) => {
                self.check_storage_dead(p)?;
            }
            repr::ActionKind::Noop => {
            }
        }

        Ok(())
    }

    fn check_write(&self, _path: &repr::Path) -> Result<(), Box<Error>> {
        Ok(())
    }

    fn check_read(&self, _path: &repr::Path) -> Result<(), Box<Error>> {
        Ok(())
    }

    /// Cannot move from a path `p` if:
    /// - `p` is borrowed;
    /// - some subpath `p.foo` is borrowed;
    /// - some prefix of `p` is borrowed.
    fn check_move(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        let prefixes = path.prefixes();
        for loan in self.loans {
            for loan_prefix in loan.path.prefixes() {
                if prefixes.contains(&loan_prefix) {
                    return Err(Box::new(BorrowError::for_move(self.point, path, &loan.path)));
                }
            }
        }
        Ok(())
    }

    fn check_storage_dead(&self, var: repr::Variable) -> Result<(), Box<Error>> {
        for loan in self.loans {
            if let Some(loan_var) = self.invalidated_by_dead_storage(&loan.path) {
                if var == loan_var {
                    return Err(Box::new(BorrowError::for_storage_dead(self.point, var, &loan.path)));
                }
            }
        }
        Ok(())
    }

    /// If `path` is borrowed, returns a vector of paths which -- if
    /// moved or if the storage went away -- would invalidate this
    /// reference.
    fn invalidated_by_dead_storage(&self, mut path: &repr::Path) -> Option<repr::Variable> {
        loop {
            match *path {
                repr::Path::Base(v) => return Some(v),
                repr::Path::Extension(ref base_path, field_name) => {
                    match *self.env.path_ty(base_path) {
                        // If you borrow `*r`, you can drop the
                        // reference `r` without invalidating that
                        // memory.
                        repr::Ty::Ref(_, _, _) => {
                            assert_eq!(field_name, repr::FieldName::star());
                            return None;
                        }

                        repr::Ty::Unit | repr::Ty::Struct(..) => {
                            path = base_path;
                        }

                        repr::Ty::Bound(..) => {
                            panic!("unexpected bound type")
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct BorrowError {
    description: String
}

impl BorrowError {
    fn no_error(point: Point) -> Self {
        BorrowError {
            description: format!("point {:?} had no error, but should have", point)
        }
    }

    fn for_move(point: Point, path: &repr::Path, loan_path: &repr::Path) -> Self {
        BorrowError {
            description: format!("point {:?} cannot move {:?} because {:?} is borrowed",
                                 point,
                                 path,
                                 loan_path)
        }
    }

    fn for_storage_dead(point: Point, var: repr::Variable, loan_path: &repr::Path) -> Self {
        BorrowError {
            description: format!("point {:?} cannot kill storage for {:?} because {:?} is borrowed",
                                 point,
                                 var,
                                 loan_path)
        }
    }
}

impl Error for BorrowError {
    fn description(&self) -> &str {
        &self.description
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.description)
    }
}
