use crossbeam_channel::{Receiver, TryRecvError};
use crate::bdd::Bdd;
use crate::bdd_util::*;
use crate::expr::bool_expr;
use crate::expr::bool_expr::Expr;
use crate::expr::bool_expr::Expr::*;
use crate::parser::parse::Dimacs;
use crate::variable_ordering::var_ordering_builder::BddVarOrderingBuilder;
use crate::statistics::stats::Stats;
use crate::GlucoseWrapper;
use crate::parallel::clause_database::ClauseDatabase;

#[derive(Clone, Debug)]
pub struct BddVarOrdering(pub std::collections::HashMap<i32, usize>);

impl BddVarOrdering {

    /// Create a new `BddVarOrdering` with the given named variables.
    pub fn new(dimacs: &Dimacs) -> BddVarOrdering {
        let mut builder = BddVarOrderingBuilder::new();
        builder.make_variables(&dimacs.vars);
        builder.make(&dimacs.vars_scores)
    }

    pub fn parallel_build(&self, vec_expr: &mut Vec<Expr>, clause_database: &mut ClauseDatabase, mut rec_depth: usize, solver_wrapper: GlucoseWrapper,
                          stats: &mut Stats, receiver1: Receiver<()>,receiver2: Receiver<()>, receiver3: Receiver<()>) -> Bdd {
        // here we are investigating 2 new clauses
        rec_depth += 2;
        let mut current_bdd = self.build(&mut vec_expr[0]);

        let mut n = 1;
        while n < vec_expr.len() {
            // check if the other thread has finished
            match receiver1.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating the Bdd.");
                    println!(" ");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            // clear the global filter every 30 clauses
            if rec_depth % 30 == 0 {
                clause_database.reset_bloom_filter_global();
            }
            // clear the local filter from former clauses
            clause_database.reset_bloom_filter_local();

            // send the current learned clauses while building the temp_bdd
            let (_, temp_bdd) = rayon::join(
                || current_bdd.send_learned_clauses(true, clause_database, solver_wrapper, stats, receiver2.clone()),
                || self.build(&mut vec_expr[n]));

            current_bdd = self.and(&current_bdd, &temp_bdd);
            // these clauses need to be added to the clauses that the bdd will investigate/process
            //let clauses_to_add = current_bdd.receive_learned_clauses( clause_database, solver_wrapper, stats);
            //self.add_clauses_during_build(vec_expr, clauses_to_add);

            // check if the other thread has finished
            match receiver1.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating the Bdd.");
                    println!(" ");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            if rec_depth % 20 == 0 {
                current_bdd.round_up(stats, receiver3.clone());
            }
            stats.add_bdd_size(current_bdd.size());

            // check if the other thread has finished
            match receiver1.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating the Bdd.");
                    println!(" ");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            rec_depth += 2;
            n+=1;
        }
        current_bdd
    }


    /// This method takes a set of clauses and adds them to the set of
    /// clauses that the Bdd processes while building the bdd.
    /// This happens each time (if there are clauses available in the
    /// set) after each exchange clause operation.
    pub fn add_clauses_during_build(&self, current_expr: &mut Vec<Expr>, clauses_to_add: Vec<Vec<i32>>) {
        // first of all we need to parse the clauses into expressions
        let mut new_expr_set = Vec::new();
        for new_clause in clauses_to_add {
            let new_expr = Expr::parse_clause(&new_clause);
            if !new_expr_set.contains(&new_expr) {
                new_expr_set.push(new_expr);
            }
        }
        let dif_set: Vec<Expr> = new_expr_set.into_iter().filter(|expr| !current_expr.contains(expr)).collect();
        current_expr.extend(dif_set);
    }

    /// Construct a Robdd from a given expression
    pub fn build(&self, expr: &mut Expr) -> Bdd {
        // The construction of a Bdd from a boolean expression proceeds
        // as in the construction of the if-then-else Normalform. An ordering
        // of the variables is fixed. Using the shannon expansion a node
        // for the expression is constructed by a call to mk (checks if
        // the exact same node exists in the node cache). The nodes for
        // each sub-expression are constructed by recursion.
        match expr {
            Const(value) => Bdd::new_value(BddVar::new(i32::MAX), value),
            Var(name) => {
                let var = BddVar::new(*name);
                Bdd::new_var(var)
            },
            Not(inner) => self.build(inner).negate(),
            And(l, r) => {
                let (left,right) = rayon::join(|| self.build(l), || self.build(r));
                self.and(&left, &right)
            },
            Or(l, r) => {
                let (left,right) = rayon::join(|| self.build(l), || self.build(r));
                self.or(&left, &right)
            }
        }
    }

    /// Create a `Bdd` corresponding to the $\phi \land \psi$ formula, where $\phi$ and $\psi$
    /// are the two given `Bdd`s.
    pub fn and(&self, left: &Bdd, right: &Bdd) -> Bdd {
        self.apply(left, right, bool_expr::and)
    }

    /// Create a `Bdd` corresponding to the $\phi \lor \psi$ formula, where $\phi$ and $\psi$
    /// are the two given `Bdd`s.
    pub fn or(&self, left: &Bdd, right: &Bdd) -> Bdd {
        self.apply(left, right, bool_expr::or)
    }

    fn apply<T>(&self, left: &Bdd, right: &Bdd, op: T) -> Bdd
        where
            T: Fn(Option<bool>, Option<bool>) -> Option<bool> {

        let mut bdd = Bdd::new();

        // In order to ensure that the Obdd being constructed is reduced,
        // it is necessary to determine from a triple (i,l,h) whether there
        // exists a node u with var(u) = i, low(u) = l and high(u) = h.
        // For this purpose we assume the presence of a table H:(i,l,h) -> u
        // mapping triples (i,h,l) of variables indices i and nodes l and h to u.
        // The table H is the "inverse" of the table T of variable nodes u.
        // T(u) = (i,l,h) if and only if H(i,l,h) = u.

        // We keep track of a nodes_map so that there are no duplicates
        let mut nodes_map: std::collections::HashMap<BddNode, BddPointer> =
            std::collections::HashMap::with_capacity(std::cmp::max(left.size(), right.size()));
        nodes_map.insert(BddNode::mk_zero(BddVar::new(i32::MAX)), BddPointer::new_zero());
        nodes_map.insert(BddNode::mk_one(BddVar::new(i32::MAX)), BddPointer::new_one());

        // Task is a pair of pointers into the `left` and `right` BDDs.
        #[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
        struct Task {
            left: BddPointer,
            right: BddPointer,
        }

        // We keep track of the tasks currently on stack so that we build the bdd from down to the top
        let mut stack: Vec<Task> = Vec::with_capacity(std::cmp::max(left.size(), right.size()));

        stack.push(Task {
            left: left.root_pointer(),
            right: right.root_pointer(),
        });

        // We keep track of the tasks already completed, so that we can access the pointers
        let mut finished_tasks: std::collections::HashMap<Task, BddPointer> =
            std::collections::HashMap::with_capacity(std::cmp::max(left.size(), right.size()));

        while let Some(current) = stack.last() {

            if finished_tasks.contains_key(current) {
                stack.pop();
            } else {
                let (lft, rgt) = (current.left, current.right);
                // find the lowest variable of the two nodes
                let (l_var, r_var) = (left.var_of_ptr(lft), right.var_of_ptr(rgt));

                //let min_var = min(l_var, r_var);
                // The min variable is now the one with the higher score, so
                // the smallest index in the mapping
                let l_var_index = self.0.get(&l_var.0).unwrap();
                let r_var_index = self.0.get(&r_var.0).unwrap();
                let min_var = if l_var_index < r_var_index { l_var } else { r_var };

                // If the nodes have the same index the two low branches are paired
                // and apply recursively computed on them. Similarly for the high branches.
                // If they have different indices we proceed by pairing the node
                // with lowest index with the low- and high- branches of the other.
                let (l_low, l_high) = if l_var.eq(&min_var) {
                    (left.low_node_ptr(lft), left.high_node_ptr(lft))
                } else {
                    (lft, lft)
                };
                let (r_low, r_high) = if l_var == r_var || r_var.eq(&min_var) {
                    (right.low_node_ptr(rgt), right.high_node_ptr(rgt))
                } else {
                    (rgt, rgt)
                };

                // Two tasks which correspond to the two recursive sub-problems we need to solve.
                let sub_left = Task {
                    left: l_low,
                    right: r_low,
                };
                let sub_right = Task {
                    left: l_high,
                    right: r_high,
                };

                let new_low: Option<BddPointer> = op(l_low.as_bool(), r_low.as_bool())
                    .map(BddPointer::from_bool)
                    .or(finished_tasks.get(&sub_left).cloned());

                let new_high: Option<BddPointer> = op(l_high.as_bool(), r_high.as_bool())
                    .map(BddPointer::from_bool)
                    .or(finished_tasks.get(&sub_right).cloned());

                if let (Some(new_low), Some(new_high)) = (new_low, new_high) {
                    if new_low == new_high {
                        finished_tasks.insert(*current, new_low);
                    } else {
                        let node = BddNode::mk_node(min_var, new_low, new_high);
                        if let Some(idx) = nodes_map.get(&node) {
                            // Node already exists, just make it a result of this computation.
                            finished_tasks.insert(*current, *idx);
                        } else {
                            // Node does not exist, it needs to be pushed to result.
                            bdd.push_node(node);
                            nodes_map.insert(node, bdd.root_pointer());
                            finished_tasks.insert(*current, bdd.root_pointer());
                        }
                    }
                    // If both values are computed, mark this task as resolved.
                    stack.pop();
                } else {
                    // add the subtasks to stack
                    if new_low.is_none() {
                        stack.push(sub_left);
                    }
                    if new_high.is_none() {
                        stack.push(sub_right);
                    }
                }
            }
        }
        bdd
    }


    /*
    pub fn parallel_build_and_send_assumptions(&self, vec_expr: &mut Vec<Expr>, clause_database: &mut ClauseDatabase, mut rec_depth: usize, solver_wrapper: GlucoseWrapper, stats: &mut Stats) -> Bdd {
        // here we are investigating 2 new clauses
        rec_depth += 2;
        let mut current_bdd = self.build(&vec_expr[0]);
        let mut n = 1;

        while n < vec_expr.len() {
            // clear the global filter every 10 clauses
            if rec_depth % 30 == 0 {
                clause_database.reset_bloom_filter_global();
            }
            // clear the local filter from former clauses
            clause_database.reset_bloom_filter_local();

            let (_, temp_bdd) = rayon::join(|| current_bdd.send_learned_clauses_to_assumptions(true, clause_database, solver_wrapper, stats),
                                            || self.build(&vec_expr[n]));

            n+=1;
            current_bdd = self.and(&current_bdd, &temp_bdd);

            if rec_depth % 10 == 0 {
                // approximation
                //current_bdd.round_up(stats);
            }
            stats.add_bdd_size(current_bdd.size());
        }
        // After finishing building the bdd
        // So no new clauses will be added to the vector of expressions to be investigated
        current_bdd.send_learned_clauses_to_assumptions(false, clause_database, solver_wrapper, stats);

        current_bdd
    }

    pub fn parallel_build_without_solver_just_for_testing(&self, vec_expr: &mut Vec<Expr>, clause_database: &mut ClauseDatabase, mut rec_depth: usize, stats: &mut Stats) -> Bdd {
        // here we are investigating 2 new clauses
        rec_depth += 2;
        let mut current_bdd = self.build(&vec_expr[0]);

        let mut n = 1;
        while n < vec_expr.len() {

            // clear the global filter every 30 clauses
            if rec_depth % 30 == 0 {
                clause_database.reset_bloom_filter_global();
            }
            // clear the local filter from former clauses
            clause_database.reset_bloom_filter_local();

            // send the current learned clauses while building the temp_bdd
            let (_, temp_bdd) = rayon::join(
                || current_bdd.send_learned_clauses_without_solver_just_for_testing(true, clause_database, stats),
                || self.build(&vec_expr[n]));

            current_bdd = self.and(&current_bdd, &temp_bdd);

            // these clauses need to be added to the clauses that the bdd will investigate/process
            //let clauses_to_add = current_bdd.receive_learned_clauses( clause_database, solver_wrapper, stats);
            //self.add_clauses_during_build(vec_expr, clauses_to_add);

            // approximation
            //if rec_depth % 10 == 0 { current_bdd.round_up(stats, receiver); }
            stats.add_bdd_size(current_bdd.size());

            rec_depth += 2;
            n+=1;
        }
        current_bdd
    }*/
}