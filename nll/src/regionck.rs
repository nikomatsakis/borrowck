use borrowck;
use env::{Environment, Point};
use loans_in_scope::LoansInScope;
use liveness::Liveness;
use infer::{InferenceContext, RegionVariable};
use nll_repr::repr::{self, RegionName, Variance};
use std::collections::HashMap;
use std::error::Error;
use region::Region;

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let ck = &mut RegionCheck {
        env,
        infer: InferenceContext::new(),
        region_map: HashMap::new(),
    };
    ck.check()
}

pub struct RegionCheck<'env> {
    env: &'env Environment<'env>,
    infer: InferenceContext,
    region_map: HashMap<repr::RegionName, RegionVariable>,
}

impl<'env> RegionCheck<'env> {
    pub fn env(&self) -> &'env Environment<'env> {
        self.env
    }

    pub fn region(&self, name: RegionName) -> &Region {
        let var = match self.region_map.get(&name) {
            Some(&var) => var,
            None => panic!("no region variable ever created with name `{:?}`", name),
        };
        self.infer.region(var)
    }

    fn check(&mut self) -> Result<(), Box<Error>> {
        let liveness = &Liveness::new(self.env);
        self.populate_inference(liveness);
        let loans_in_scope = &LoansInScope::new(self);
        borrowck::borrow_check(self.env, loans_in_scope)?;
        self.check_assertions(liveness)?;
        Ok(())
    }

    fn check_assertions(&self, liveness: &Liveness) -> Result<(), Box<Error>> {
        let mut errors = 0;

        for assertion in self.env.graph.assertions() {
            match *assertion {
                repr::Assertion::Eq(region_name, ref region_literal) => {
                    let region_var = self.region_map[&region_name];
                    let region_value = self.to_region(region_literal);
                    if *self.infer.region(region_var) != region_value {
                        errors += 1;
                        println!("error: region variable `{:?}` has wrong value", region_name);
                        println!("  expected: {:?}", region_value);
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::In(region_name, ref point) => {
                    let region_var = self.region_map[&region_name];
                    let point = self.to_point(point);
                    if !self.infer.region(region_var).may_contain(point) {
                        errors += 1;
                        println!(
                            "error: region variable `{:?}` does not contain `{:?}`",
                            region_name,
                            point
                        );
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::NotIn(region_name, ref point) => {
                    let region_var = self.region_map[&region_name];
                    let point = self.to_point(point);
                    if self.infer.region(region_var).may_contain(point) {
                        errors += 1;
                        println!(
                            "error: region variable `{:?}` contains `{:?}`",
                            region_name,
                            point
                        );
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::Live(var, block_name) => {
                    let block = self.env.graph.block(block_name);
                    if !liveness.var_live_on_entry(var, block) {
                        errors += 1;
                        println!(
                            "error: variable `{:?}` not live on entry to `{:?}`",
                            var,
                            block_name
                        );
                    }
                }

                repr::Assertion::NotLive(var, block_name) => {
                    let block = self.env.graph.block(block_name);
                    if liveness.var_live_on_entry(var, block) {
                        errors += 1;
                        println!(
                            "error: variable `{:?}` live on entry to `{:?}`",
                            var,
                            block_name
                        );
                    }
                }
            }
        }

        if errors > 0 {
            try!(Err(format!("{} errors found", errors)));
        }

        Ok(())
    }

    fn populate_inference(&mut self, liveness: &Liveness) {
        liveness.walk(self.env, |point, action, live_on_entry| {
            // To start, find every variable `x` that is live. All regions
            // in the type of `x` must include `point`.
            for region_name in liveness.live_regions(live_on_entry) {
                let rv = self.region_variable(region_name);
                self.infer.add_live_point(rv, point);
            }

            let action = if let Some(action) = action {
                action
            } else {
                return;
            };

            // Next, walk the actions and establish any additional constraints
            // that may arise from subtyping.
            let successor_point = Point {
                block: point.block,
                action: point.action + 1,
            };
            match action.kind {
                // `p = &'x` -- first, `'x` must include this point @ P,
                // and second `&'x <: typeof(p) @ succ(P)`
                repr::ActionKind::Borrow(
                    ref dest_path,
                    region_name,
                    borrow_kind,
                    ref source_path,
                ) => {
                    let dest_ty = self.env.path_ty(dest_path);
                    let source_ty = self.env.path_ty(source_path);
                    let ref_ty = Box::new(repr::Ty::Ref(
                        repr::Region::Free(region_name),
                        borrow_kind,
                        source_ty,
                    ));
                    self.relate_tys(successor_point, repr::Variance::Contra, &dest_ty, &ref_ty);
                    self.ensure_borrow_source(successor_point, region_name, source_path);
                }

                // a = b
                repr::ActionKind::Assign(ref a, ref b) => {
                    let a_ty = self.env.path_ty(a);
                    let b_ty = self.env.path_ty(b);

                    // `b` must be a subtype of `a` to be assignable:
                    self.relate_tys(successor_point, repr::Variance::Co, &b_ty, &a_ty);
                }

                // 'X: 'Y
                repr::ActionKind::Constraint(ref c) => {
                    match **c {
                        repr::Constraint::Outlives(c) => {
                            let sup_v = self.region_variable(c.sup);
                            let sub_v = self.region_variable(c.sub);
                            self.infer.add_outlives(sup_v, sub_v, point);
                        }
                        _ => {
                            panic!("unimplemented rich constraint: {:?}", c);
                        }
                    }
                }

                repr::ActionKind::Init(..) |
                repr::ActionKind::Use(..) |
                repr::ActionKind::Drop(..) |
                repr::ActionKind::StorageDead(..) |
                repr::ActionKind::Noop => {
                    // no add'l constriants needed here; basic liveness
                    // suffices.
                }
            }
        });

        self.infer.solve(self.env);
    }

    fn region_variable(&mut self, n: repr::RegionName) -> RegionVariable {
        let infer = &mut self.infer;
        let r = *self.region_map.entry(n).or_insert_with(|| infer.add_var());
        log!("{:?} => {:?}", n, r);
        r
    }

    fn to_point(&self, point: &repr::Point) -> Point {
        let block = self.env.graph.block(point.block);
        Point {
            block: block,
            action: point.action,
        }
    }

    fn to_region(&self, user_region: &repr::RegionLiteral) -> Region {
        let mut region = Region::new();
        for p in &user_region.points {
            region.add_point(self.to_point(p));
        }
        region
    }

    fn relate_tys(
        &mut self,
        successor_point: Point,
        variance: repr::Variance,
        a: &repr::Ty,
        b: &repr::Ty,
    ) {
        log!(
            "relate_tys({:?} {:?} {:?} @ {:?})",
            a,
            variance,
            b,
            successor_point
        );
        match (a, b) {
            (&repr::Ty::Ref(r_a, bk_a, ref t_a), &repr::Ty::Ref(r_b, bk_b, ref t_b)) => {
                assert_eq!(bk_a, bk_b, "cannot relate {:?} and {:?}", a, b);
                self.relate_regions(
                    successor_point,
                    variance.invert(),
                    r_a.assert_free(),
                    r_b.assert_free(),
                );
                let referent_variance = variance.xform(bk_a.variance());
                self.relate_tys(successor_point, referent_variance, t_a, t_b);
            }
            (&repr::Ty::Unit, &repr::Ty::Unit) => {}
            (&repr::Ty::Struct(s_a, ref ps_a), &repr::Ty::Struct(s_b, ref ps_b)) => {
                if s_a != s_b {
                    panic!("cannot compare `{:?}` and `{:?}`", s_a, s_b);
                }
                let s_decl = self.env.struct_map[&s_a];
                if ps_a.len() != s_decl.parameters.len() {
                    panic!("wrong number of parameters for `{:?}`", a);
                }
                if ps_b.len() != s_decl.parameters.len() {
                    panic!("wrong number of parameters for `{:?}`", b);
                }
                for (sp, (p_a, p_b)) in s_decl.parameters.iter().zip(ps_a.iter().zip(ps_b)) {
                    let v = variance.xform(sp.variance);
                    self.relate_parameters(successor_point, v, p_a, p_b);
                }
            }
            _ => {
                panic!(
                    "cannot relate types `{:?}` and `{:?}` at {:?}",
                    a,
                    b,
                    successor_point
                )
            }
        }
    }

    fn relate_regions(
        &mut self,
        successor_point: Point,
        variance: repr::Variance,
        a: repr::RegionName,
        b: repr::RegionName,
    ) {
        log!(
            "relate_regions({:?} {:?} {:?} @ {:?})",
            a,
            variance,
            b,
            successor_point
        );
        let r_a = self.region_variable(a);
        let r_b = self.region_variable(b);
        match variance {
            Variance::Co =>
                // "a Co b" == "a <= b"
                self.infer.add_outlives(r_b, r_a, successor_point),
            Variance::Contra =>
                // "a Contra b" == "a >= b"
                self.infer.add_outlives(r_a, r_b, successor_point),
            Variance::In => {
                self.infer.add_outlives(r_a, r_b, successor_point);
                self.infer.add_outlives(r_b, r_a, successor_point);
            }
        }
    }

    fn relate_parameters(
        &mut self,
        successor_point: Point,
        variance: repr::Variance,
        a: &repr::TyParameter,
        b: &repr::TyParameter,
    ) {
        match (a, b) {
            (&repr::TyParameter::Ty(ref t_a), &repr::TyParameter::Ty(ref t_b)) => {
                self.relate_tys(successor_point, variance, t_a, t_b)
            }
            (&repr::TyParameter::Region(r_a), &repr::TyParameter::Region(r_b)) => {
                self.relate_regions(
                    successor_point,
                    variance,
                    r_a.assert_free(),
                    r_b.assert_free(),
                )
            }
            _ => panic!("cannot relate parameters `{:?}` and `{:?}`", a, b),
        }
    }

    /// Add any relations between regions that are needed to ensures
    /// that reborrows live long enough. Specifically, if we borrow
    /// something like `*r` for `'a`, where `r: &'b i32`, then `'b:
    /// 'a` is required.
    fn ensure_borrow_source(
        &mut self,
        successor_point: Point,
        borrow_region_name: RegionName,
        source_path: &repr::Path,
    ) {
        log!(
            "ensure_borrow_source({:?}, {:?}, {:?})",
            successor_point,
            borrow_region_name,
            source_path
        );

        for supporting_path in self.env.supporting_prefixes(source_path) {
            match *supporting_path {
                repr::Path::Var(_) => {
                    // No lifetime constraints are needed to ensure the
                    // validity of a variable. That is ensured by borrowck
                    // preventing the storage of variables from being killed
                    // while data owned by that variable is in scope.
                    return;
                }
                repr::Path::Extension(ref base_path, field_name) => {
                    let ty = self.env.path_ty(base_path);
                    log!("ensure_borrow_source: ty={:?}", ty);
                    match *ty {
                        repr::Ty::Ref(ref_region, _, _) => {
                            assert_eq!(field_name, repr::FieldName::star());
                            let ref_region_name = ref_region.assert_free();
                            let borrow_region_variable = self.region_variable(borrow_region_name);
                            let ref_region_variable = self.region_variable(ref_region_name);
                            self.infer.add_outlives(
                                ref_region_variable,
                                borrow_region_variable,
                                successor_point,
                            );
                        }
                        repr::Ty::Unit => {}
                        repr::Ty::Struct(..) => {}
                        repr::Ty::Bound(..) => {}
                    }
                }
            }
        }
    }
}
