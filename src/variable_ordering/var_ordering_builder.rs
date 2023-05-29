use std::cmp::Ordering;
use std::cmp::Ordering::*;
use crate::bdd_util::BddVar;
use crate::variable_ordering::var_ordering::BddVarOrdering;

#[derive(Clone, Debug)]
pub struct BddVarOrderingBuilder {
    var_names: Vec<i32>,
    var_names_set: std::collections::HashSet<i32>,
}

impl BddVarOrderingBuilder {
    /// Create a new builder without any variables.
    pub fn new() -> BddVarOrderingBuilder {
        BddVarOrderingBuilder {
            var_names: Vec::new(),
            var_names_set: std::collections::HashSet::new(),
        }
    }

    /// Create a new variable with the given `name`. Returns a `BddVar`
    /// instance that can be later used to create and query actual BDDs.
    ///
    /// *Panics*:
    ///  - Each variable name has to be unique.
    pub fn make_variable(&mut self, name: &i32) -> BddVar {
        if self.var_names_set.contains(name) {
            panic!("BDD variable {} already exists.", name);
        }
        self.var_names_set.insert(*name);
        self.var_names.push(*name);
        BddVar(*name)
    }


    /// Similar to `make_variable`, but allows creating multiple variables at the same time.
    pub fn make_variables(&mut self, names: &Vec<i32>) -> Vec<BddVar> {
        names.iter().map(|name| self.make_variable(name)).collect()
    }

    /// Convert this builder to an actual variable ordering.
    /// The variables are sorted in decreasing order according to the score,
    /// so that higher-scoring variables
    /// (that is, variables that appear in many mostly short clauses)
    /// correspond to layers nearer the top of the BDD.
    pub fn make(self, vars_scores: &std::collections::HashMap<i32, f64>) -> BddVarOrdering {
        let mut mapping: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        let mut v: Vec<_> = vars_scores.into_iter().collect();
        // v is a sorted vector in decreasing order according to the scores
        v.sort_by(|x,y| BddVarOrderingBuilder::var_dec_cmp(&x.1, &y.1));
        let mut idx = 0;
        for (var, _) in v {
            mapping.insert(*var, idx);
            idx += 1;
        }
        mapping.insert(i32::MAX, idx);

        BddVarOrdering(mapping)
    }

    fn var_dec_cmp(x: &f64, y: &f64) -> Ordering {
        if x.eq(&y) {
            Equal
        } else if x < y {
            Greater
        } else {
            Less
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::parser::parse::parse_dimacs;
    use super::*;

    #[test]
    fn variable_scores() {
        let dimacs = parse_dimacs("tests/test3.cnf");
        let vars_scores = dimacs.vars_scores;
        // score for 1:
        // number of clauses containing the var: 6
        // average arity of those clauses: (5+2+2+2+2+2) / 5 = 2,5
        // score = 6/2.5 = 2,4

        assert_eq!(*vars_scores.get(&1).unwrap(), 2.4 as f64);
        assert!(vars_scores.get(&1).unwrap() > vars_scores.get(&5).unwrap());
    }

    #[test]
    fn variable_ordering() {
        let dimacs = parse_dimacs("tests/test3.cnf");
        let var_ordering = BddVarOrdering::new(&dimacs);

        let mut var_index_mapping: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        var_index_mapping.insert(1,0);
        var_index_mapping.insert(2,1);
        var_index_mapping.insert(3,2);
        var_index_mapping.insert(4,3);
        var_index_mapping.insert(5,4);
        var_index_mapping.insert(i32::MAX,5);

        assert_eq!(var_index_mapping, var_ordering.0);
    }
}