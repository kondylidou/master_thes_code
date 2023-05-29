use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};

// Nodes are represented as numbers with 0 and 1 reserved for the terminal nodes.
#[derive(Clone, Debug, Eq, Copy, Hash)]
pub struct BddPointer(pub u32);

impl BddPointer {

    pub fn new(index: usize) -> BddPointer {
        BddPointer(index as u32)
    }

    pub fn new_zero() -> BddPointer {
        BddPointer(0)
    }

    pub fn new_one() -> BddPointer { BddPointer(1) }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn is_one(&self) -> bool { self.0 == 1 }

    pub fn is_terminal(&self) -> bool {
        self.0 < 2
    }

    pub fn rename(&mut self, new: u32) { self.0 = new; }

    pub fn from_bool(value: bool) -> BddPointer {
        if value {
            BddPointer::new_one()
        } else {
            BddPointer::new_zero()
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.0 {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    pub fn to_index(self) -> usize {
        self.0 as usize
    }

    pub fn flip_if_terminal(&mut self) {
        if self.0 < 2 {
            self.0 = (self.0 + 1) % 2;
        }
    }
}

impl std::fmt::Display for BddPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}


impl PartialEq for BddPointer {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug, Clone, Eq, Copy, Ord, Hash)]
pub struct BddVar(pub i32);

impl BddVar {

    pub fn new(name: i32) -> Self {
        BddVar(name)
    }

    fn repr(&self) -> String {
        format!("x{}", self.0)
    }

    // The name can be the same in many variables as we have ordered BDDs but not yet reduced.
    // It's better to get a unique pointer for each variable.
    pub unsafe fn addr(&self) -> usize {
        // get a reference to the value
        let var_ref = &self as *const _;
        // convert the reference to a raw pointer
        let var_raw_ptr = var_ref as *const u64;
        // convert the raw pointer to an integer
        let var_addr = var_raw_ptr as usize;
        var_addr
    }
}

impl std::fmt::Display for BddVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.repr())
    }
}

impl PartialEq for BddVar {
    // As two variables can have the same name they are only
    // then equal if the pointers match.
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for BddVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0.eq(&other.0) {
            Some(Equal)
        } else if self.0 < other.0 {
            Some(Less)
        } else {
            Some(Greater)
        }
    }
}


/// A [BddNode] is representing one Node in the decision diagram.
///
/// Intuitively this is a binary tree structure, where the diagram is allowed to
/// pool same values to the same Node.
///
/// Each subexpression can be viewed as the node of a graph. Such a node is either
/// terminal in the case of constants 0 and 1 or non-terminal.
#[derive(Eq, Clone, Copy, Hash)]
pub struct BddNode {
    pub var: BddVar, // TODO we need a unique pointer on the variable as variable names are not unique
    pub low: BddPointer,
    pub high: BddPointer
}

impl BddNode {

    pub fn mk_value(var: BddVar, value: &bool) -> BddNode {
        if *value { BddNode::mk_one(var) } else { BddNode::mk_zero(var) }
    }

    pub fn mk_zero(var: BddVar) -> BddNode {
        let zero_leaf = BddPointer::new_zero();
        BddNode {
            var,
            low: zero_leaf,
            high: zero_leaf,
        }
    }

    pub fn flip_zero(&mut self) {
        self.low = BddPointer::new_one();
        self.high = BddPointer::new_zero();
    }

    pub fn mk_one(var: BddVar) -> BddNode {
        let one_leaf = BddPointer::new_one();
        BddNode {
            var,
            low: one_leaf,
            high: one_leaf,
        }
    }

    pub fn flip_one(&mut self) {
        self.low = BddPointer::new_zero();
        self.high = BddPointer::new_one();
    }

    pub fn mk_node(var: BddVar, low: BddPointer, high: BddPointer) -> BddNode {
        BddNode {
            var,
            low,
            high
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.low.eq(&self.high) && (self.low.eq(&BddPointer::new_one()) || self.low.eq(&BddPointer::new_zero()))
    }
    pub fn is_one(&self) -> bool {
        self.is_terminal() && self.low.eq(&BddPointer::new_one())
    }
    pub fn is_zero(&self) -> bool {
        self.is_terminal() && self.low.eq(&BddPointer::new_zero())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.is_terminal() {
            true => {if self.is_zero() { Some(false)} else {Some(true)}},
            _ => None,
        }
    }

    pub fn not(&mut self) -> BddNode {
        self.low.flip_if_terminal();
        self.high.flip_if_terminal();
        *self
    }

    pub fn replace_low(&mut self, new: BddPointer) {
        self.low = new;
    }

    pub fn replace_high(&mut self, new: BddPointer) {
        self.high = new;
    }

    pub fn decrease_low(&mut self) {
        let cur_low = self.low;
        let new_low = BddPointer::new(cur_low.to_index() - 1);
        self.low = new_low;
    }

    pub fn decrease_high(&mut self) {
        let cur_high = self.high;
        let new_high = BddPointer::new(cur_high.to_index() - 1);
        self.high = new_high;
    }
}

impl std::fmt::Debug for BddNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let var = self.var;
        let value = format!(" → {},{}", self.low, self.high);
        write!(f, "{}{}", var, value)
    }
}

impl PartialEq for BddNode {
    fn eq(&self, other: &Self) -> bool {
        // variable equality by pointer equality
        let res_var = self.var == other.var;
        // bindings are same
        let res_low = self.low == other.low;
        let res_high = self.high == other.high;

        res_var && (res_low && res_high)
    }
}

impl std::fmt::Display for BddNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut s = format!("{} -> ", self.var);
        s.push('(');
        s.push_str(&format!("{}", self.low));
        s.push_str(", ");
        s.push_str(&format!("{}", self.high));
        s.push(')');
        write!(f, "{}", s)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdd_node_one() {
        let x1 = BddVar::new(1);
        let one = BddNode::mk_one(x1);

        assert!(one.is_terminal());
        assert!(one.is_one());
        assert!(!one.is_zero());
        assert_eq!(x1, one.var);

    }

    #[test]
    fn bdd_node_zero() {
        let x1 = BddVar::new(1);
        let zero = BddNode::mk_zero(x1);

        assert!(zero.is_terminal());
        assert!(zero.is_zero());
        assert!(!zero.is_one());
        assert_eq!(x1, zero.var);

    }

    #[test]
    fn bdd_node_create() {
        let x1 = BddVar::new(1);
        let x2 = BddVar::new(2);

        let _node2 = BddNode::mk_node(
            x2,
            BddPointer::new_one(),
            BddPointer::new_one(),
        );

        let node2_pointer = BddPointer::new(2);

        let node1 = BddNode::mk_node(
            x1,
            node2_pointer,
            BddPointer::new_one(),
        );

        assert_eq!(node2_pointer, node1.low);
        assert_eq!(BddPointer::new_one(), node1.high);
        assert_eq!(x1, node1.var);
        assert!(!node1.is_terminal());
        assert!(!node1.is_one());
        assert!(!node1.is_zero());
    }
}