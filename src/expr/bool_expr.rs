use std::collections::HashMap;
use crate::expr::bool_expr::Expr::*;
use std::ops::Not;

/// Recursive implementation of boolean expression.
/// Firstly only the ones important for SAT Solving clauses
/// will be considered. It can be extended afterwards.
#[derive(Clone, Debug, Eq)]
pub enum Expr {
    Const(bool),
    Var(i32),
    Not(Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
}

impl Expr {

    pub fn new_var(var: i32) -> Expr {
        Expr::Var(var)
    }

    pub fn get_right(&self) -> Option<&Box<Expr>> {
        match self {
            Or(_,r) => Some(r),
            And(_,r) => Some(r),
            _ => None
        }
    }

    pub fn get_left(&self) -> Option<&Box<Expr>> {
        match self {
            Or(l,_) => Some(l),
            And(l,_) => Some(l),
            _ => None
        }
    }


    pub fn parse_clauses(clauses: &Vec<Vec<i32>>) -> Vec<Expr> {
        let mut vec_expr = Vec::new();
        if clauses.len() == 1 {
            vec_expr.push(Expr::parse_clause(&clauses[0]));
            return vec_expr;
        }

        let mut acc = 0;
        let mut n = 2;
        while n <= clauses.len() {
            let clauses_pair = clauses[acc..n].to_vec();
            vec_expr.push(Expr::And(Box::new(Expr::parse_clause(&clauses_pair[0])),
                                    Box::new(Expr::parse_clause(&clauses_pair[1]))));
            acc = n;
            n+=2;
        }
        if clauses.len() % 2 != 0 {
            vec_expr.push(Expr::And(Box::new(Expr::parse_clause(&clauses[clauses.len()-1])),
                                    Box::new(Expr::Const(true))));
        }
        vec_expr
    }

    pub fn parse_clause(clause: &Vec<i32>) -> Expr {
        if clause.len() == 1 {
            return Expr::parse_var(&clause[0]);
        }
        Expr::Or(Box::new(Expr::parse_var(&clause[0])),
                 Box::new(Expr::parse_clause(&clause[1..].to_vec())))
    }

    pub fn parse_var(var: &i32) -> Expr {
        if var.to_string().chars().nth(0).unwrap() == '-' {
            let val = &var.to_string()[1..];
            Expr::Not(Box::new(Var(val.parse().unwrap())))
        } else {
            Expr::Var(*var)
        }
    }

    pub fn set_vars_and_solve(&self, assignment: &HashMap<i32, bool>) -> Option<bool> {
        match self {
            Const(val) => Some(*val),
            Var(name) => {
                if let Some(valuet) = assignment.get(name) {
                    Some(*valuet)
                } else if let Some(valuef) = assignment.get(&-name) {
                    Some(valuef.not())
                }
                else { None }
            },
            Not(inner) => {
                match inner.set_vars_and_solve(assignment) {
                    Some(val) => { if val { Some(false) } else { Some(true) } }
                    None => None
                }
            },
            And(l, r) => {
                let left = l.set_vars_and_solve(assignment);
                let right = r.set_vars_and_solve(assignment);
                and(left,right)
            },
            Or(l, r) => {
                let left = l.set_vars_and_solve(assignment);
                let right = r.set_vars_and_solve(assignment);
                or(left,right)
            }
        }
    }

    pub fn step(&self) -> Expr {
        let newval = match self {
            Expr::And(x, y) => {
                let (x, y) = (*x.clone(), *y.clone());
                match (x, y) {
                    (Expr::Const(false), _) => Expr::Const(false),
                    (Expr::Const(true), a) => a.step(),
                    (_, Expr::Const(false)) => Expr::Const(false),
                    (a, Expr::Const(true)) => a.step(),
                    (Expr::Or(a, b), c) => {
                        let l = Expr::And(a, Box::new(c.clone())).step();
                        let r = Expr::And(b, Box::new(c)).step();
                        Expr::Or(Box::new(l), Box::new(r))
                    },
                    (c, Expr::Or(a, b)) => {
                        let l = Expr::And(Box::new(c.clone()), a).step();
                        let r = Expr::And(Box::new(c), b).step();
                        Expr::Or(Box::new(l), Box::new(r))
                    },
                    (x, y) => {
                        if x == y {
                            x.step()
                        } else {
                            Expr::And(Box::new(x.step()), Box::new(y.step()))
                        }
                    }
                }
            }
            Expr::Or(x, y) => {
                let (x, y) = (*x.clone(), *y.clone());
                match (x, y) {
                    (Expr::Const(true), _) => Expr::Const(true),
                    (Expr::Const(false), a) => a.step(),
                    (_, Expr::Const(true)) => Expr::Const(true),
                    (a, Expr::Const(false)) => a.step(),
                    (x, y) => {
                        if x == y {
                            x.step()
                        } else {
                            Expr::Or(Box::new(x.step()), Box::new(y.step()))
                        }
                    }
                }
            }
            Expr::Not(x) => {
                match &**x {
                    Expr::Const(false) => Expr::Const(true),
                    Expr::Const(true) => Expr::Const(false),
                    Expr::And(a, b) => Expr::Or(
                        Box::new(Expr::Not(a.clone()).step()),
                        Box::new(Expr::Not(b.clone()).step()),
                    ),
                    Expr::Or(a, b) => Expr::And(
                        Box::new(Expr::Not(a.clone()).step()),
                        Box::new(Expr::Not(b.clone()).step()),
                    ),
                    Expr::Not(a) => a.step(),
                    x => {
                        Expr::Not(Box::new(x.step()))
                    }
                }
            }
            Expr::Var(t) => Expr::Var(*t),
            Expr::Const(c) => Expr::Const(*c),
        };
        newval
    }
}


impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Const(value) => write!(f, "{}", value),
            Var(name) => write!(f, "{}", name),
            Not(inner) => write!(f, "!{}", inner),
            And(l, r) => write!(f, "({} & {})", l, r),
            Or(l, r) => write!(f, "({} | {})", l, r)
        }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        match (self,other) {
            (Const(val1), Const(val2)) => val1 == val2,
            (Var(name1), Var(name2)) => name1 == name2,
            (Not(in1) , Not(in2) ) => in1 == in2,
            (And(l1, r1), And(l2, r2)) => (l1 == l2) && (r1 == r2),
            (Or(l1, r1), Or(l2, r2)) => (l1 == l2) && (r1 == r2),
            _ => false
        }
    }
}

/// Partial operator function corresponding to $x \land y$.
pub fn and(l: Option<bool>, r: Option<bool>) -> Option<bool> {
    match (l, r) {
        (Some(true), Some(true)) => Some(true),
        (Some(false), _) => Some(false),
        (_, Some(false)) => Some(false),
        _ => None,
    }
}

/// Partial operator function corresponding to $x \lor y$.
pub fn or(l: Option<bool>, r: Option<bool>) -> Option<bool> {
    match (l, r) {
        (Some(false), Some(false)) => Some(false),
        (Some(true), _) => Some(true),
        (_, Some(true)) => Some(true),
        _ => None,
    }
}


#[cfg(test)]
mod tests {
    use crate::expr::bool_expr::Expr;

    #[test]
    pub fn test_parse_var1() {
        let var1 = -1;
        let var2 = 2;
        let var3 = 3;

        let res_var1 = Expr::Not(Box::new(Expr::Var(1)));
        let res_var2 = Expr::Var(2);
        let res_var3 = Expr::Var(3);

        let parsed_var1 = Expr::parse_var(&var1);
        let parsed_var2 = Expr::parse_var(&var2);
        let parsed_var3 = Expr::parse_var(&var3);

        assert_eq!(parsed_var1, res_var1);
        assert_eq!(parsed_var2, res_var2);
        assert_eq!(parsed_var3, res_var3);
    }

    #[test]
    pub fn test_parse_or1() {
        let clause = vec![-1,2,3];

        let parsed_clause = Expr::parse_clause(&clause);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));
        let var2 = Expr::Var(2);
        let var3 = Expr::Var(3);

        let res_cla = Expr::Or(Box::new(var1),
                               Box::new(Expr::Or(Box::new(var2), Box::new(var3))));

        assert_eq!(parsed_clause, res_cla);
    }

    #[test]
    pub fn test_parse_or2() {
        let clause = vec![-1,2,3,4];

        let parsed_clause = Expr::parse_clause(&clause);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));
        let var2 = Expr::Var(2);
        let var3 = Expr::Var(3);
        let var4 = Expr::Var(4);

        let res_cla = Expr::Or(Box::new(var1),
                               Box::new(Expr::Or(Box::new(var2),
                                                 Box::new(Expr::Or(Box::new(var3),
                                                                   Box::new(var4))))));

        assert_eq!(parsed_clause, res_cla);
    }

    #[test]
    pub fn test_parse_or3() {
        let clause = vec![-1,2,3,4,-5];

        let parsed_clause = Expr::parse_clause(&clause);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));
        let var2 = Expr::Var(2);
        let var3 = Expr::Var(3);
        let var4 = Expr::Var(4);
        let var5 = Expr::Not(Box::new(Expr::Var(5)));

        let res_cla = Expr::Or(
            Box::new(var1), Box::new(Expr::Or(
                Box::new(var2), Box::new(Expr::Or(
                    Box::new(var3), Box::new(Expr::Or(
                        Box::new(var4), Box::new(var5))))))));

        assert_eq!(parsed_clause, res_cla);
    }

    #[test]
    pub fn test_parse_boolexpr_1() {
        let mut clauses = Vec::new();
        let clause1 = vec![-1,2,3];
        let clause2 = vec![4,-5,6];
        let clause3 = vec![7,8,-9];

        clauses.push(clause1);
        clauses.push(clause2);
        clauses.push(clause3);

        let parsed_clauses = Expr::parse_clauses(&clauses);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));
        let var2 = Expr::Var(2);
        let var3 = Expr::Var(3);
        let var4 = Expr::Var(4);
        let var5 = Expr::Not(Box::new(Expr::Var(5)));
        let var6 = Expr::Var(6);
        let var7 = Expr::Var(7);
        let var8 = Expr::Var(8);
        let var9 = Expr::Not(Box::new(Expr::Var(9)));

        let cla1 = Expr::Or(Box::new(var1),
                            Box::new(Expr::Or(Box::new(var2), Box::new(var3))));
        let cla2 = Expr::Or(Box::new(var4),
                            Box::new(Expr::Or(Box::new(var5), Box::new(var6))));
        let cla3 = Expr::Or(Box::new(var7),
                            Box::new(Expr::Or(Box::new(var8), Box::new(var9))));

        let res_clauses = vec![Expr::And(Box::new(cla1),Box::new(cla2)),
            Expr::And(Box::new(cla3),Box::new(Expr::Const(true)))];

        assert_eq!(parsed_clauses, res_clauses);
    }
    #[test]
    pub fn test_parse_boolexpr_2() {
        let mut clauses = Vec::new();
        let clause1 = vec![-1,2,3,4];
        let clause2 = vec![-5,6];
        let clause3 = vec![7,8,-9];
        let clause4 = vec![10,11,12,13];
        let clause5 = vec![14];

        clauses.push(clause1);
        clauses.push(clause2);
        clauses.push(clause3);
        clauses.push(clause4);
        clauses.push(clause5);

        let parsed_clauses = Expr::parse_clauses(&clauses);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));
        let var2 = Expr::Var(2);
        let var3 = Expr::Var(3);
        let var4 = Expr::Var(4);
        let var5 = Expr::Not(Box::new(Expr::Var(5)));
        let var6 = Expr::Var(6);
        let var7 = Expr::Var(7);
        let var8 = Expr::Var(8);
        let var9 = Expr::Not(Box::new(Expr::Var(9)));
        let var10 = Expr::Var(10);
        let var11 = Expr::Var(11);
        let var12 = Expr::Var(12);
        let var13 = Expr::Var(13);
        let var14 = Expr::Var(14);

        let cla1 = Expr::Or(Box::new(var1), Box::new(
            Expr::Or(Box::new(var2), Box::new(
                Expr::Or(Box::new(var3), Box::new(var4))))));
        let cla2 = Expr::Or(Box::new(var5), Box::new(var6));
        let cla3 = Expr::Or(Box::new(var7),
                            Box::new(Expr::Or(Box::new(var8), Box::new(var9))));
        let cla4 = Expr::Or(Box::new(var10), Box::new(
            Expr::Or(Box::new(var11), Box::new(
                Expr::Or(Box::new(var12), Box::new(var13))))));
        let cla5 = var14;


        let res_clauses = vec![Expr::And(Box::new(cla1),Box::new(cla2)),
                               Expr::And(Box::new(cla3),Box::new(cla4)),
                               Expr::And(Box::new(cla5),Box::new(Expr::Const(true)))];

        assert_eq!(parsed_clauses, res_clauses);
    }

    #[test]
    pub fn test_parse_boolexpr_3() {
        let mut clauses = Vec::new();
        let clause1 = vec![-1];

        clauses.push(clause1);

        let parsed_clauses = Expr::parse_clauses(&clauses);

        let var1 = Expr::Not(Box::new(Expr::Var(1)));

        let cla1 = var1;
        let res_clauses = vec![cla1];

        assert_eq!(parsed_clauses, res_clauses);
    }
}