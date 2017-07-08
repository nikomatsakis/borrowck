use def_use::DefUse;
use env::{Environment, Point};
use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::bit_set::{BitBuf, BitSet, BitSlice};
use nll_repr::repr;
use regionck::RegionCheck;
use std::collections::HashMap;

pub struct LoansInScope<'rck> {
    regionck: &'rck RegionCheck<'rck>,
    loans: Vec<Loan<'rck>>,
    loans_in_scope_after_block: BitSet<FuncGraph>,
    loans_by_point: HashMap<Point, usize>,
}

pub struct Loan<'rck> {
    point: Point,
    path: &'rck repr::Path,
    kind: repr::BorrowKind,
    region: repr::RegionName,
}

impl<'rck> LoansInScope<'rck> {
    pub fn new(regionck: &'rck RegionCheck<'rck>) -> Self {
        let env = regionck.env();

        // Collect the full set of loans; these are just the set of
        // `&foo` expressions.
        let loans: Vec<_> =
            env.reverse_post_order
               .iter()
               .flat_map(|&block| {
                   env.graph.block_data(block)
                            .actions
                            .iter()
                            .enumerate()
                            .flat_map(move |(index, action)| match *action {
                                repr::Action::Borrow(_, region, kind, ref path) => {
                                    let point = Point { block, action: index };
                                    Some(Loan { point, region, kind, path })
                                }

                                _ => None,
                            })
               })
               .collect();

        // Make a convenient hash map for getting the index of a loan
        // based on where it appears.
        let loans_by_point: HashMap<_, _> =
            loans.iter()
                 .enumerate()
                 .map(|(index, loan)| (loan.point, index))
                 .collect();

        // Get a bit set with the set of in-scope loans at each point
        // in the graph. These correspond to the set of loans in scope
        // at the end of the block.
        let loans_in_scope_after_block = BitSet::new(env.graph, loans.len());

        // iterate until fixed point
        let mut this = LoansInScope { regionck, loans, loans_by_point, loans_in_scope_after_block };
        this.compute();

        this
    }

    pub fn env(&self) -> &'rck Environment<'rck> {
        self.regionck.env()
    }

    /// Invokes `callback` with the loans in scope at each point.
    pub fn walk<CB>(&self, mut callback: CB)
        where CB: FnMut(Point, Option<&repr::Action>, &[&Loan])
    {
        let env = self.env();
        let mut loans = Vec::with_capacity(self.loans.len());
        let mut bits = self.loans_in_scope_after_block.empty_buf();
        for &block in &env.reverse_post_order {
            self.simulate_block(env, &mut bits, block, |point, action, bits| {
                // Convert from the bitset into a vector of references to loans.
                loans.clear();
                loans.extend(
                    self.loans.iter()
                              .enumerate()
                              .filter_map(|(loan_index, loan)| {
                                  if bits.get(loan_index) {
                                      Some(loan)
                                  } else {
                                      None
                                  }
                              }));

                // Invoke the callback.
                callback(point, action, &loans);
            });
        }
    }

    /// Iterates until a fixed point, computing the loans in scope
    /// after each block terminates.
    fn compute(&mut self) {
        let env = self.env();
        let mut bits = self.loans_in_scope_after_block.empty_buf();
        let mut changed = true;
        while changed {
            changed = false;

            for &block in &env.reverse_post_order {
                self.simulate_block(env, &mut bits, block, |_p, _a, _s| ());
                changed |= self.loans_in_scope_after_block
                               .insert_bits_from_slice(block, bits.as_slice());
            }
        }
    }

    fn simulate_block<CB>(&self,
                          env: &Environment,
                          buf: &mut BitBuf,
                          block: BasicBlockIndex,
                          mut callback: CB)
        where CB: FnMut(Point, Option<&repr::Action>, BitSlice)
    {
        buf.clear();

        // everything live at end of a pred  is live at the exit of the block
        for succ in env.graph.successors(block) {
            buf.set_from(self.loans_in_scope_after_block.bits(succ));
        }

        // walk through the actions on by one
        for (index, action) in env.graph.block_data(block).actions.iter().enumerate() {
            let point = Point { block, action: index };

            // kill any loans where `point` is not in their region
            for loan_index in self.loans_not_in_scope_at(point) {
                buf.kill(loan_index);
            }

            // callback at start of the action
            callback(point, Some(action), buf.as_slice());

            // bring the loan into scope after the borrow
            if let Some(&loan_index) = self.loans_by_point.get(&point) {
                buf.set(loan_index);
            }

            // figure out which path is overwritten by this action;
            // this may cancel out some loans
            if let Some(overwritten_path) = action.overwrites() {
                for loan_index in self.loans_killed_by_write_to(overwritten_path) {
                    buf.kill(loan_index);
                }
            }
        }

        // final callback for the terminator
        let point = env.end_point(block);
        for loan_index in self.loans_not_in_scope_at(point) {
            buf.kill(loan_index);
        }
        callback(point, None, buf.as_slice());
    }

    fn loans_not_in_scope_at<'a>(&'a self, point: Point)
                                 -> impl Iterator<Item = usize> + 'a
    {
        self.loans.iter()
                  .enumerate()
                  .filter_map(move |(loan_index, loan)| {
                      let region = self.regionck.region(loan.region);
                      if !region.contains(point) {
                          Some(loan_index)
                      } else {
                          None
                      }
                  })
    }

    fn loans_killed_by_write_to<'a>(&'a self, path: &'a repr::Path)
                                         -> impl Iterator<Item = usize> + 'a
    {
        // When an assignment like `a.b.c = ...` occurs, we kill all
        // the loans for `a.b.c` or some subpath like `a.b.c.d`, since
        // the path no longer evaluates to the same thing.
        self.loans.iter()
                  .enumerate()
                  .filter_map(move |(index, loan)| {
                      if self.path_prefixes(loan.path).iter().any(|&p| p == path) {
                          Some(index)
                      } else {
                          None
                      }
                  })
    }

    fn path_prefixes<'a>(&self, mut path: &'a repr::Path) -> Vec<&'a repr::Path> {
        let mut result = vec![];
        loop {
            result.push(path);
            match *path {
                repr::Path::Base(_) => return result,
                repr::Path::Extension(ref base, _) => path = base,
            }
        }
    }
}

