use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use pest_derive::*;
use pest::Parser;

#[derive(Parser)]
#[grammar = "parser/dimacs_grammar.pest"]
struct DimacsParser;

#[derive(Clone, Debug)]
pub struct Dimacs {
    pub nb_v: usize,
    pub nb_c: usize,
    pub vars: Vec<i32>,
    pub vars_scores: HashMap<i32, f64>,
    pub clauses: Vec<Vec<i32>>,
}

pub fn parse_dimacs(path: &str) -> Dimacs {
    let input = fs::read_to_string(path)
        .expect("Something went wrong reading the file");

    let tok = DimacsParser::parse(Rule::file, &input).map_err(|e| format!("parser error: {}", e));
    let tok = tok.unwrap().next().unwrap();

    let mut nb_v = 0;
    let mut nb_c = 0;
    let mut vars = Vec::new();
    let mut clauses = Vec::new();

    // this hashmap contains a variable and the arities of the clauses where
    // this variable is appearing.
    let mut var_clause_arities: HashMap<i32, Vec<usize>> = HashMap::new();

    for pair in tok.into_inner() {
        match pair.as_rule() {
            Rule::n => { nb_v = pair.as_str().parse().unwrap(); }
            Rule::m => { nb_c = pair.as_str().parse().unwrap(); }
            Rule::clause => {
                let mut clause: Vec<i32> = Vec::new();
                let mut clause_vars: Vec<i32> = Vec::new();
                for lit in pair.into_inner() {
                    let mut val: &str = lit.as_str();
                    clause.push(val.parse().unwrap());
                    if val.chars().nth(0).unwrap() == '-' {
                        val = &val[1..];
                    }
                    let var: i32 = val.parse().unwrap();
                    if !vars.contains(&var) {
                        vars.push(var);
                    }
                    clause_vars.push(var);
                }
                // add the clause arity to each variable appearing in this clause
                for var in clause_vars {
                    if let Some(arities) = var_clause_arities.get_mut(&var) {
                        arities.push(clause.len());
                    } else {
                        var_clause_arities.insert(var, vec![clause.len()]);
                    }
                }
                clauses.push(clause);
            }
            Rule::EOI => (),
            _ => unreachable!(),
        }
    }
    let vars_scores = calculate_score(var_clause_arities);
    Dimacs {
        nb_v,
        nb_c,
        vars,
        vars_scores,
        clauses,
    }
}

/// For our implementation, we use a simple heuristic to determine the variable ordering:
/// each variable is assigned a score, computed as the quotient between the number of clauses
/// containing the variable and the average arity of those clauses.
pub fn calculate_score(var_clause_arities: HashMap<i32, Vec<usize>>) -> HashMap<i32, f64> {
    let mut vars_scores = HashMap::new();
    for (var, clause_arities) in var_clause_arities {
        // the number of clauses where the variable appears
        let clauses_num = clause_arities.len() as f64;
        // the average arity of those clauses is computed by dividing
        // the sum of the arities with the total number of clauses
        let sum: usize = clause_arities.iter().sum();
        let aver_arity = sum as f64 / clauses_num;
        // the score is computed as the quotient between the number of clauses
        // containing the variable and the average arity of those clauses
        let score = clauses_num / aver_arity;
        vars_scores.insert(var, score);
    }
    // TODO write tests on vars score
    vars_scores
}


#[cfg(test)]
mod tests {
    use crate::parser::parse::parse_dimacs;

    #[test]
    pub fn test_parser() {
        let input: &str = "tests/test1.cnf";

        let mut vars = Vec::new();
        let mut clauses = Vec::new();

        vars.push(83);
        vars.push(16);
        vars.push(65);
        vars.push(188);
        vars.push(1);
        vars.push(171);
        vars.push(23);
        vars.push(132);
        vars.push(59);

        //-83 16 65 0
        // 188 1 171 0
        // 23 132 -59 0

        clauses.push(vec![-83,16,65]);
        clauses.push(vec![188,1,171]);
        clauses.push(vec![23,132,-59]);

        let parsed = parse_dimacs(input);
        assert_eq!(clauses, parsed.clauses);
        assert_eq!(vars, parsed.vars);
    }
}