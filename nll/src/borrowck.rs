use env::{Environment, Point};
use loans_in_scope::{Loan, LoansInScope};
use nll_repr::repr;
use std::error::Error;
use std::fmt;

pub fn borrow_check(env: &Environment, loans_in_scope: &LoansInScope) -> Result<(), Box<Error>> {
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

enum Depth {
    Shallow,
    Deep,
}

enum Mode {
    Read,
    Write,
}

impl<'cx> BorrowCheck<'cx> {
    fn check_action(&self, action: &repr::Action) -> Result<(), Box<Error>> {
        log!("check_action({:?}) at {:?}", action, self.point);
        match action.kind {
            repr::ActionKind::Init(ref a, ref bs) => {
                self.check_shallow_write(a)?;
                for b in bs {
                    self.check_read(b)?;
                }
            }
            repr::ActionKind::Assign(ref a, ref b) => {
                self.check_shallow_write(a)?;
                self.check_read(b)?;
            }
            repr::ActionKind::Borrow(ref a, _, repr::BorrowKind::Shared, ref b) => {
                self.check_shallow_write(a)?;
                self.check_read(b)?;
            }
            repr::ActionKind::Borrow(ref a, _, repr::BorrowKind::Mut, ref b) => {
                self.check_shallow_write(a)?;
                self.check_mut_borrow(b)?;
            }
            repr::ActionKind::Constraint(_) => {}
            repr::ActionKind::Use(ref p) => {
                self.check_read(p)?;
            }
            repr::ActionKind::Drop(ref p) => {
                self.check_drop(p)?;
            }
            repr::ActionKind::StorageDead(p) => {
                self.check_storage_dead(p)?;
            }
            repr::ActionKind::SkolemizedEnd(_) |
            repr::ActionKind::Noop => {}
        }

        Ok(())
    }

    /// `use(x)` may access `x` and (by going through the produced
    /// value) anything reachable from `x`.
    fn check_read(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        self.check_borrows(Depth::Deep, Mode::Read, path)
    }

    /// `x = ...` overwrites `x` (without reading it) and prevents any
    /// further reads from that path.
    fn check_shallow_write(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        self.check_borrows(Depth::Shallow, Mode::Write, path)
    }

    /// `&mut x` may mutate `x`, but it can also *read* from `x`, and
    /// mutate things reachable from `x`.
    fn check_mut_borrow(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        self.check_borrows(Depth::Deep, Mode::Write, path)
    }

    fn check_borrows(&self,
                     depth: Depth,
                     access_mode: Mode,
                     path: &repr::Path)
                     -> Result<(), Box<Error>> {
        let loans: Vec<_> = match depth {
            Depth::Shallow => self.find_loans_that_freeze(path).collect(),
            Depth::Deep => self.find_loans_that_intersect(path).collect(),
        };

        for loan in loans {
            match access_mode {
                Mode::Read => match loan.kind {
                    repr::BorrowKind::Shared => { /* Ok */ }
                    repr::BorrowKind::Mut => {
                        return Err(Box::new(BorrowError::for_read(
                            self.point,
                            path,
                            &loan.path,
                            loan.point,
                        )));
                    }
                },

                Mode::Write => {
                    return Err(Box::new(BorrowError::for_write(
                        self.point,
                        path,
                        &loan.path,
                        loan.point,
                    )));
                },
            }
        }

        Ok(())
    }

    /// Cannot drop (*) for a path `p` if:
    /// - `p` is borrowed;
    /// - some subpath `p.foo` is borrowed (unless *every* projection
    ///   for the subpath is may_dangle)
    /// - some prefix of `p` is borrowed
    ///
    /// Note that the above disjunction is stricter than both *writes*
    /// and *storage-dead*. In particular, you **can** write to a variable
    /// `x` that contains an `&mut value when `*x` is borrowed, but you
    /// **cannot** drop `x`. This is because the drop may run a destructor
    /// that could subsequently access `*x` via the variable.
    ///
    /// (On the other hand, `may_dangle` throws a wrench into the
    /// reasoning above. Namely, even if `*x` is borrowed, you still
    /// **can** drop `x` that contains a `&'l mut value` where
    /// `may_dangle 'l`, because that serves as a flag that the
    /// destructor is not allowed to access the data behind any
    /// reference of lifetime `'l`.)
    ///
    /// (*): to drop is to check the initialization-flag (be it static
    /// or dynamic), and run all destructors recursively if
    /// initialized)
    fn check_drop(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        log!(
            "check_drop of {:?} at {:?} with loans={:#?}",
            path,
            self.point,
            self.loans
        );
        for loan in self.find_loans_that_intersect(path) {
            return Err(Box::new(BorrowError::for_drop(
                self.point,
                path,
                &loan.path,
                loan.point,
            )));
        }
        Ok(())
    }

    #[cfg(not_now)]
    /// Cannot move from a path `p` if:
    /// - `p` is borrowed;
    /// - some subpath `p.foo` is borrowed;
    /// - some prefix of `p` is borrowed.
    ///
    /// Note that this is stricter than both *writes* and
    /// *storage-dead*. In particular, you **can** write to a variable
    /// `x` that contains an `&mut` value when `*x` is borrowed, but
    /// you **cannot** move `x`. This is because moving it would make
    /// the `&mut` available in the new location, but writing (and
    /// storage-dead) both kill it forever.
    fn check_move(&self, path: &repr::Path) -> Result<(), Box<Error>> {
        log!(
            "check_move of {:?} at {:?} with loans={:#?}",
            path,
            self.point,
            self.loans
        );
        for loan in self.find_loans_that_intersect(path) {
            return Err(Box::new(BorrowError::for_move(
                self.point,
                path,
                &loan.path,
                loan.point,
            )));
        }
        Ok(())
    }

    /// Cannot free a local variable `var` if:
    /// - data interior to `var` is borrowed.
    ///
    /// In particular, having something like `*var` borrowed is ok.
    fn check_storage_dead(&self, var: repr::Variable) -> Result<(), Box<Error>> {
        log!(
            "check_storage_dead of {:?} at {:?} with loans={:#?}",
            var,
            self.point,
            self.loans
        );
        for loan in self.find_loans_that_freeze(&repr::Path::Var(var)) {
            return Err(Box::new(BorrowError::for_storage_dead(
                self.point,
                var,
                &loan.path,
                loan.point,
            )));
        }
        Ok(())
    }

    /// A loan L *intersects* a path P if either:
    ///
    /// - the loan is for the path P; or,
    /// - the path P can be extended to reach the data in the loan; or,
    /// - the loan path can be extended to reach the data in P.
    ///
    /// So, for example, is the path P is `a.b.c`, then:
    ///
    /// - a loan of `a.b.c` intersects P;
    /// - a loan of `a.b.c.d` intersects P, because (e.g.) after reading P
    ///   you have also read `a.b.c.d`;
    /// - a loan of `a.b` intersects P, because you can use the
    ///   reference to access the data at P.
    fn find_loans_that_intersect<'a>(
        &'a self,
        path: &'a repr::Path,
    ) -> impl Iterator<Item = &'a Loan> + 'a {
        let path_prefixes = path.prefixes();
        self.loans.iter().cloned().filter(move |loan| {
            // accessing `a.b.c` intersects a loan of `a.b.c` or `a.b`...
            path_prefixes.contains(&loan.path) ||

            // ...as well as a loan of `a.b.c.d`
                self.env.supporting_prefixes(&loan.path).contains(&path)
        })
    }

    /// Helper for `check_write` and `check_storage_dead`: finds if
    /// there is a loan that "freezes" the given path -- that is, a
    /// loan that would make modifying the `path` (or freeing it)
    /// illegal. This is slightly more permissive than the rules
    /// around move and reads, precisely because overwriting or
    /// freeing `path` makes the previous value unavailable from that
    /// point on.
    fn find_loans_that_freeze<'a>(
        &'a self,
        path: &repr::Path)
        -> impl Iterator<Item = &'a Loan> + 'a
    {
        let path: repr::Path = path.clone();
        self.loans.iter().cloned().filter(move |loan| {
            let prefixes = path.prefixes();

            // If you have borrowed `a.b`, this prevents writes to `a`
            // or `a.b`:
            let frozen_paths = self.frozen_by_borrow_of(&loan.path);
            frozen_paths.contains(&&path) ||

                // If you have borrowed `a.b`, this prevents writes to
                // `a.b.c`:
                prefixes.contains(&loan.path)
        })
    }

    /// If `path` is mutably borrowed, returns a vector of paths which -- if
    /// moved or if the storage went away -- would invalidate this
    /// reference.
    fn frozen_by_borrow_of<'a>(&self, mut path: &'a repr::Path) -> Vec<&'a repr::Path> {
        let mut result = vec![];
        loop {
            result.push(path);
            match *path {
                repr::Path::Var(_) => return result,
                repr::Path::Extension(ref base_path, field_name) => {
                    match *self.env.path_ty(base_path) {
                        // If you borrowed `*r`, writing to `r` does
                        // not actually affect the memory at `*r`, so
                        // we can stop iterating backwards now.
                        repr::Ty::Ref(_, _, _) => {
                            assert_eq!(field_name, repr::FieldName::star());
                            return result;
                        }

                        // If you have borrowed `a.b`, then writing to
                        // `a` would overwrite `a.b`, which is
                        // disallowed.
                        repr::Ty::Struct(..) => {
                            path = base_path;
                        }

                        repr::Ty::Unit => panic!("unit has no fields"),
                        repr::Ty::Bound(..) => panic!("unexpected bound type"),
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct BorrowError {
    description: String,
}

impl BorrowError {
    fn no_error(point: Point) -> Self {
        BorrowError {
            description: format!("point {:?} had no error, but should have", point),
        }
    }

    fn for_drop(
        point: Point,
        path: &repr::Path,
        loan_path: &repr::Path,
        loan_point: Point,
    ) -> Self {
        BorrowError {
            description: format!(
                "point {:?} cannot drop `{}` because `{}` is borrowed (at point `{:?}`)",
                point,
                path,
                loan_path,
                loan_point
            ),
        }
    }

    #[cfg(not_now)]
    fn for_move(
        point: Point,
        path: &repr::Path,
        loan_path: &repr::Path,
        loan_point: Point,
    ) -> Self {
        BorrowError {
            description: format!(
                "point {:?} cannot move {:?} because {:?} is borrowed (at point `{:?}`)",
                point,
                path,
                loan_path,
                loan_point
            ),
        }
    }

    fn for_read(
        point: Point,
        path: &repr::Path,
        loan_path: &repr::Path,
        loan_point: Point,
    ) -> Self {
        BorrowError {
            description: format!(
                "point {:?} cannot read {:?} because {:?} is mutably borrowed (at point `{:?}`)",
                point,
                path,
                loan_path,
                loan_point
            ),
        }
    }

    fn for_write(
        point: Point,
        path: &repr::Path,
        loan_path: &repr::Path,
        loan_point: Point,
    ) -> Self {
        BorrowError {
            description: format!(
                "point {:?} cannot write {:?} because {:?} is borrowed (at point `{:?}`)",
                point,
                path,
                loan_path,
                loan_point
            ),
        }
    }

    fn for_storage_dead(
        point: Point,
        var: repr::Variable,
        loan_path: &repr::Path,
        loan_point: Point,
    ) -> Self {
        BorrowError {
            description: format!(
                "point {:?} cannot kill storage for {:?} \
                 because {:?} is borrowed (at point `{:?}`)",
                point,
                var,
                loan_path,
                loan_point
            ),
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
