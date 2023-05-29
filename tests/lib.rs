use bdd_sat_solver::{get_glucose_solution, get_glucose_solution_no_malloc, init_glucose_solver, parse_dimacs_and_add_clause_to_glucose, run_glucose};

#[test]
pub fn test_solver_get_solution_1(){
    let solver = init_glucose_solver();
    let nb_v = parse_dimacs_and_add_clause_to_glucose("benchmarks/tests/sgen4-unsat-65-1.cnf".to_string(), solver);
    let ret = run_glucose(solver);
    match ret {
        0 => {
            let _sol = get_glucose_solution(solver, nb_v);
        },
        _ => println!("Solution assertion failed."),
    }
}

#[test]
pub fn test_solver_get_solution_2(){
    let solver = init_glucose_solver();
    let nb_v = parse_dimacs_and_add_clause_to_glucose("benchmarks/tests/sgen4-unsat-65-1.cnf".to_string(), solver);
    let ret = run_glucose(solver);
    match ret {
        0 => {
            let mut sol = Vec::with_capacity(nb_v);
            get_glucose_solution_no_malloc(solver, &mut sol, nb_v);
        },
        _ => println!("Solution assertion failed."),
    }
}