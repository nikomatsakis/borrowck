use env::{Environment, Point};
use loans_in_scope::{Loan, LoansInScope};
use nll_repr::repr;
use std::error::Error;
use std::fmt;

pub fn borrow_check(env: &Environment,
                    loans_in_scope: &LoansInScope)
                    -> Result<(), Box<Error>> {
    let mut result = Ok(());
    loans_in_scope.walk(env, |point, opt_action, loans| {
        let borrowck = BorrowCheck { point, loans };
        if let Some(action) = opt_action {
            if let Err(e) = borrowck.check_action(action) {
                if !action.should_have_error {
                    result = Err(e);
                }
            }
        }
    });

    result
}

struct BorrowCheck<'cx> {
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
            repr::ActionKind::Drop(ref p) | repr::ActionKind::StorageDead(ref p) => {
                self.check_move(p)?;
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
}

#[derive(Debug)]
pub struct BorrowError {
    description: String
}

impl BorrowError {
    fn for_move(point: Point, path: &repr::Path, loan_path: &repr::Path) -> Self {
        BorrowError {
            description: format!("point {:?} cannot move {:?} because {:?} is borrowed",
                                 point,
                                 path,
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
