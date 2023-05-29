use std::collections::HashMap;
use std::iter::Map;
use std::ops::Range;
use rand::Rng;
use crate::bdd_util::{BddNode, BddPointer, BddVar};
use rand::seq::SliceRandom;
use crate::expr::bool_expr::Expr;

// The Bdd receives the clauses 'Vec<Vec<i32>>'. They can be viewed as a boolean
// expression for example (x1 OR x2) AND (NOT x1 OR x2). Then the INF (if then else normalform)
// needs to be found for this expression so that the Bdd can be constructed.

#[derive(Clone, Debug, Eq)]
pub struct Bdd(pub Vec<BddNode>);

impl Bdd {

    /// Create a new empty Bdd. The terminal pointers are
    /// inserted into the vector of nodes.
    pub fn new() -> Bdd {
        let mut nodes = Vec::new();
        // Maximum number as pointer as in the apply method always the smaller var is
        // selected and we want to replace these nodes.
        let max_ptr = BddVar::new(i32::MAX);
        nodes.push(BddNode::mk_zero(max_ptr));
        nodes.push(BddNode::mk_one(max_ptr));
        Bdd(nodes)
    }

    pub fn new_with_capacity(cap: usize) -> Bdd {
        let mut nodes = Vec::with_capacity(cap);
        // Maximum number as pointer as in the apply method always the smaller var is
        // selected and we want to replace these nodes.
        let max_ptr = BddVar::new(i32::MAX);
        nodes.push(BddNode::mk_zero(max_ptr));
        nodes.push(BddNode::mk_one(max_ptr));
        Bdd(nodes)
    }

    pub fn is_full(&self) -> bool {
        self.0.capacity().eq(&self.0.len())
    }

    pub fn is_empty(&self) -> bool {
        self.size().eq(&2)
    }

    /// Get the variable of a specific pointer in the Bdd.
    pub fn var_of_ptr(&self, ptr: BddPointer) -> BddVar {
        self.0[ptr.to_index()].var
    }

    /// Insert a node into the vector of nodes of the Bdd.
    pub fn push_node(&mut self, node: BddNode) {
        self.0.push(node);
    }

    /// Create a new Bdd from a variable and connect it to terminal pointers 0 and 1.
    pub fn new_var(var: BddVar) -> Bdd {
        let mut bdd = Bdd::new();
        bdd.push_node(BddNode::mk_node(var, BddPointer::new_zero(), BddPointer::new_one()));
        bdd
    }

    /// Create a new Bdd from a boolean value.
    pub fn new_value(var: BddVar, value: &bool) -> Bdd {
        if *value { Bdd::new_true(var) } else { Bdd::new_false(var) }
    }

    /// Create a new Bdd for the false formula.
    pub fn new_false(var: BddVar) -> Bdd {
        let mut bdd = Bdd::new();
        bdd.push_node(BddNode::mk_zero(var));
        bdd
    }

    /// Create a new Bdd for the true formula.
    pub fn new_true(var: BddVar) -> Bdd {
        let mut bdd = Bdd::new();
        bdd.push_node(BddNode::mk_zero(var));
        bdd.push_node(BddNode::mk_one(var));
        bdd
    }

    /// Create a new Bdd for a negated variable.
    pub fn new_not_var(var: BddVar) -> Bdd {
        let mut bdd = Bdd::new();
        bdd.push_node(BddNode::mk_node(var, BddPointer::new_one(), BddPointer::new_zero()));
        bdd
    }

    /// Negate a Bdd.
    pub fn negate(&mut self) -> Bdd {
        if self.is_true() {
            Bdd::new_false(BddVar::new(i32::MAX))
        } else if self.is_false() {
            Bdd::new_true(BddVar::new(i32::MAX))
        } else {
            let mut nodes = self.0.clone();
            for node in nodes.iter_mut().skip(2) {
                // skip terminals
                node.high.flip_if_terminal();
                node.low.flip_if_terminal();
            }
            Bdd(nodes)
        }
    }

    /// The number of nodes in a Bdd.
    pub fn size(&self) -> usize { self.0.len() }

    /// True if a Bdd is exactly the true formula.
    pub fn is_true(&self) -> bool { self.0.len() == 2 }

    /// True if a Bdd is exactly the false formula.
    pub fn is_false(&self) -> bool { self.0.len() == 1 }

    /// Get the pointer of the root node of the Bdd.
    pub fn root_pointer(&self) -> BddPointer {
        if self.is_false() {
            BddPointer::new_zero()
        } else if self.is_true() {
            BddPointer::new_one()
        } else {
            BddPointer::new(self.0.len() - 1)
        }
    }

    pub fn indices(&self) -> Map<Range<usize>, fn(usize) -> BddPointer> {
        (0..self.size()).map(BddPointer::new)
    }

    pub fn low_node_ptr(&self, ptr: BddPointer) -> BddPointer { self.0[ptr.to_index()].low }

    pub fn replace_low(&mut self, ptr: BddPointer, new_ptr: BddPointer) { self.0[ptr.to_index()].low = new_ptr }

    pub fn high_node_ptr(&self, ptr: BddPointer) -> BddPointer {
        self.0[ptr.to_index()].high
    }

    pub fn replace_high(&mut self, ptr: BddPointer, new_ptr: BddPointer) { self.0[ptr.to_index()].high = new_ptr }

    pub fn delete_node(&mut self, to_delete: BddPointer, node_path: Vec<(BddPointer,bool)>) {
        self.0.remove(to_delete.to_index());
        // the path until the node to delete was reached
        for (node, assign) in node_path.into_iter().skip(1) { // skip the first one as it was already assigned
            if assign { // if true then decrement the high nodes
                self.replace_high(node, BddPointer(self.high_node_ptr(node).0-1));
            } else { // if false then decrement the low nodes
                self.replace_low(node, BddPointer(self.low_node_ptr(node).0-1));
            }
        }
    }

    pub fn replace_node(&mut self, to_delete: BddPointer, replacement: BddPointer) {
        self.0.remove(to_delete.to_index());
        for ptr in self.indices() {
            if self.low_node_ptr(ptr).eq(&to_delete) {
                self.replace_low(ptr, replacement);
            } else if self.high_node_ptr(ptr).eq(&to_delete) {
                self.replace_high(ptr, replacement);
            }
        }
    }

    /// Check if the Bdd is satisfiable and if its the case return
    /// the satisfiable assignment in a vector of bool.
    pub fn solve(&self, ordered_vars: &Vec<i32>) -> Result<HashMap<i32, bool>, &str> {
        // If the Bdd is false return None.
        if self.is_false() {
            return Err("The problem is not solvable!");
        }
        // Initialise the final assignment with a capacity of the total number of variables.
        let mut assignment: HashMap<i32, bool> = HashMap::with_capacity(ordered_vars.len() as usize);
        let mut acc = BddPointer::new_one();

        // Search the Bdd backwards starting from the one pointer.
        for ptr in self.indices() {

            if ptr.is_terminal() {
                // skip the terminal nodes
                continue;
            }
            if self.low_node_ptr(ptr) == acc {
                // push front as we go backwards and assign the variables
                // from the last to the first.
                let var = self.var_of_ptr(ptr).0;
                assignment.insert(var, false);
                acc = ptr;
            }
            if self.high_node_ptr(ptr) == acc {
                let var = self.var_of_ptr(ptr).0;
                assignment.insert(var, true);
                // save the new pointer in the accumulator.
                acc = ptr;
            }
        }

        Ok(assignment)
    }

    /// Randomly choose clauses from the set of clauses and check if the found assignment satisfies them.
    pub fn check_sat(&self, ordered_vars: &Vec<i32>, clauses_set: &Vec<Vec<i32>>, clauses_count: usize) -> Result<bool, &'static str> {
        let assignment = self.solve(&ordered_vars);
        match assignment {
            Ok(mut sat) => {
                // If variables are not set its because a non canonical bdd is formed.
                // These variables appear in two clause once not negated and once negated.
                // By resolution they are deleted from both clauses as they are always true.
                for var in ordered_vars {
                    if !sat.contains_key(var) && !sat.contains_key(&-var) {
                        // it is not important what polarity these variables have
                        sat.insert(*var, false);
                    }
                }
                let amount;
                if clauses_count > 1 {
                    let mut rng = rand::thread_rng();
                    amount = rng.gen_range(1..clauses_count);
                } else { amount = 1; }
                let sample_clauses: Vec<_> = clauses_set.choose_multiple(&mut rand::thread_rng(), amount).cloned().collect();
                let sample_vec_expr = Expr::parse_clauses(&sample_clauses);
                for sample_expr in sample_vec_expr {
                    match sample_expr.set_vars_and_solve(&sat) {
                        Some(value) => if !value { return Err("The assignment is false!") },
                        None => return Err("Not sufficient information in the assignment!")
                    }
                }
                Ok(true)
            },
            Err(err) => panic!("{}", err)
        }
    }
}

impl PartialEq for Bdd {
    fn eq(&self, other: &Self) -> bool {
        (self.size() == other.size()) &&
            (self.0.iter().all(|x| other.0.contains(x))) &&
            (other.0.iter().all(|y| self.0.contains(y)))

    }
}