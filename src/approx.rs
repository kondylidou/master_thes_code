use std::collections::HashMap;
use std::time::Instant;
use crossbeam_channel::{Receiver, TryRecvError};
use crate::bdd::Bdd;
use crate::bdd_util::{BddNode, BddPointer};
use crate::statistics::stats::Stats;

impl Bdd {

    /// Find the node with the smallest off-set for the
    /// approximation algorithm rounding-up. An off-set
    /// is a set of the paths leading to 0.
    pub fn off_set(&self) -> Option<BddPointer> {
        if self.is_true() {
            return None;
        }
        if self.is_false() {
            return Some(self.root_pointer());
        }
        // We are searching for the off-set
        let zero = BddPointer::new_zero();
        let one = BddPointer::new_one();
        let mut off_set: HashMap<BddPointer, i32> = HashMap::new();

        // Search the Bdd backwards starting from the zero pointer.
        for ptr in self.indices() {

            if ptr.is_terminal() {
                // skip the terminal nodes
                continue;
            }
            if self.low_node_ptr(ptr) == zero && self.high_node_ptr(ptr) == one {
                let freq = off_set.get(&ptr).unwrap_or(&0);
                off_set.insert(ptr, freq + 1);
            }
            if self.high_node_ptr(ptr) == zero && self.low_node_ptr(ptr) == one {
                let freq = off_set.get(&ptr).unwrap_or(&0);
                off_set.insert(ptr, freq + 1);
            }
        }
        let mut off_set_vec: Vec<_> = off_set.into_iter().collect();
        off_set_vec.sort_by(|a, b| b.1.cmp(&a.1));
        /*
        for (ptr,_) in off_set {
            if self.var_of_ptr(ptr).0 > var_acc.0 {
                var_acc = self.var_of_ptr(ptr);
                res_ptr = ptr;
            }
        }
        */
        Some(off_set_vec[0].0)
    }

    pub fn round_up(&mut self, stats: &mut Stats, receiver: Receiver<()>) {
        let now = Instant::now();
        if let Some(ptr) = self.off_set() {
            if self.low_node_ptr(ptr).is_zero() {
                // set the low node of the pointer to true
                self.replace_low(ptr, BddPointer::new_one());
            } else {
                self.replace_high(ptr, BddPointer::new_one());
            }

            let mut parents = self.update(ptr, BddPointer::new_one());

            while let Some(idx) = parents.pop() {
                match receiver.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        println!("Terminating the approximation.");
                        println!(" ");
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
                parents.extend(self.update(BddPointer::new(idx), BddPointer::new_one()));
            }
        }
        stats.add_t_approx(now.elapsed());
    }

    pub fn update(&mut self, pointer: BddPointer, replace: BddPointer) -> Vec<usize> {
        // the pointer is a terminal one pointer now
        // and needs to be removed after all its parents redirected to one
        self.0.remove(pointer.to_index());

        let mut existing: HashMap<BddNode, BddPointer> = HashMap::new();
        let mut to_replace: Vec<(usize,BddPointer)> = Vec::new();

        // find the parent nodes and update them
        let mut parents = Vec::new();

        for (idx, node) in self.0.iter_mut().enumerate() {
            // The low pointer of the node will be replaced with one
            if node.low == pointer {
                node.replace_low(replace);
                // If this update makes the node pointer the one pointer
                // it goes to the list of parents for the recursive procedure
                if node.high.is_one() {
                    parents.push(idx);
                }
            }
            if node.high == pointer {
                node.replace_high(replace);
                if node.low.is_one() {
                    parents.push(idx);
                }
            }
            //each node that points to a pointer with an index higher
            // than the one which was deleted, it should be decreased by one
            if node.low.to_index() > pointer.to_index() {
                node.decrease_low();
            }
            if node.high.to_index() > pointer.to_index() {
                node.decrease_high();
            }
            if node.low == node.high && !node.is_terminal() {
                // if the high and low pointer of a node are the same delete
                // this node too. Of-course the node shouldn't be a terminal node
                to_replace.push((idx, node.low));
            }
            // if the node as its formed already exists, it should be replaced
            if let Some(i) = existing.get(node) {
                if !to_replace.contains(&(idx, *i)) {
                    to_replace.push((idx, *i));
                }
            }
            existing.insert(*node, BddPointer::new(idx));
        }

        // make the deletes
        self.delete(to_replace);

        parents
    }

    pub fn delete(&mut self, mut to_replace: Vec<(usize, BddPointer)>) {
        while let Some((r,n)) = to_replace.pop() {
            self.0.remove(r);

            let mut existing: HashMap<BddNode, BddPointer> = HashMap::new();

            // replace the nodes whose low or high pointers point at the node pointer
            // with the new pointer
            let ptr = BddPointer::new(r);
            for (idx, node) in self.0.iter_mut().enumerate() {
                if node.low == ptr {
                    node.replace_low(n);
                }
                if node.high == ptr {
                    node.replace_high(n);
                }
                if node.low.to_index() > r {
                    node.decrease_low();
                }
                if node.high.to_index() > r {
                    node.decrease_high();
                }
                if node.low == node.high && !node.is_terminal() {
                    // if the high and low pointer of a node are the same delete
                    // this node too. Ofcourse the node shouldn't be a terminal node
                    to_replace.push((idx, node.low));
                }
                // if the node as its formed already exists, it should be replaced
                if let Some(i) = existing.get(node) {
                    if !to_replace.contains(&(idx, *i)) {
                        to_replace.push((idx, *i));
                    }
                }
                existing.insert(*node, BddPointer::new(idx));
            }
        }
    }

    pub fn tauto_reduction(&mut self) {
        if self.is_true() || self.is_false() {
            return;
        }
        let last_child = self.root_pointer();

        let path = Vec::new();
        self.tauto_reduction_rec(path,last_child);
    }

    pub fn tauto_reduction_rec(&mut self, path: Vec<(BddPointer,bool)>, mut last_child: BddPointer) {

        if !self.low_node_ptr(last_child).is_terminal() {
            let mut pathlow = Vec::new();
            pathlow.extend(path.clone());

            // add the variable of the top level and its evaluation
            // to the path so that if the variable is encountered again
            // the node will be marked as redundant.
            pathlow.push((last_child, false)); // false as we are in the low node

            let curr_low = self.low_node_ptr(last_child);
            // now begin to search for redundant nodes

            if pathlow.iter().any(|tuple| self.var_of_ptr(tuple.0)
                == self.var_of_ptr(curr_low) && tuple.1 == false) {
                // connect the low of the last child with the low of the current
                // node which is redundant and will be removed.
                self.replace_low(last_child, self.low_node_ptr(curr_low));
                self.delete_node(curr_low, pathlow);
                last_child = BddPointer::new(last_child.0 as usize -1);
            } else {
                self.tauto_reduction_rec(pathlow, curr_low);
            }
        }
        if !self.high_node_ptr(last_child).is_terminal() {

            let mut pathhigh = Vec::new();
            pathhigh.extend(path);

            // add the variable of the top level and its evaluation
            // to the path so that if the variable is encountered again
            // the node will be marked as redundant.
            pathhigh.push((last_child, true)); // true as we are in the high node

            let curr_high = self.high_node_ptr(last_child);
            // now begin to search for redundant nodes
            if pathhigh.iter().any(|tuple| self.var_of_ptr(tuple.0)
                == self.var_of_ptr(curr_high) && tuple.1 == true) {
                // connect the high of the last child with the high of the current
                // node which is redundant and will be removed.
                self.replace_high(last_child, self.high_node_ptr(curr_high));
                self.delete_node(curr_high, pathhigh);
                //last_child = BddPointer::new(last_child.0 as usize -1);
            } else {
                self.tauto_reduction_rec(pathhigh, curr_high);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::bdd::Bdd;
    use crate::bdd_util::{BddNode, BddPointer, BddVar};


    #[test]
    pub fn test_tauto_red_1() {

        let node2 = BddNode::mk_node(BddVar(1), BddPointer(0), BddPointer(1));
        let node3 = BddNode::mk_node(BddVar(2), BddPointer(0), BddPointer(2));
        let node4 = BddNode::mk_node(BddVar(1), BddPointer(0), BddPointer(3));


        let mut bdd = Bdd::new();
        bdd.push_node(node2);
        bdd.push_node(node3);
        bdd.push_node(node4);

        bdd.tauto_reduction();
    }
    #[test]
    pub fn test_tauto_red_2() {

        let node2 = BddNode::mk_node(BddVar(1), BddPointer(1), BddPointer(0));
        let node3 = BddNode::mk_node(BddVar(1), BddPointer(0), BddPointer(1));
        let node4 = BddNode::mk_node(BddVar(3), BddPointer(2), BddPointer(1));
        let node5 = BddNode::mk_node(BddVar(2), BddPointer(4), BddPointer(3));
        let node6 = BddNode::mk_node(BddVar(1), BddPointer(4), BddPointer(0));
        let node7 = BddNode::mk_node(BddVar(1), BddPointer(6), BddPointer(5));


        let mut bdd = Bdd::new();
        bdd.push_node(node2);
        bdd.push_node(node3);
        bdd.push_node(node4);
        bdd.push_node(node5);
        bdd.push_node(node6);
        bdd.push_node(node7);

        bdd.tauto_reduction();
    }
}