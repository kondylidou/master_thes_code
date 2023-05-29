use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};
use cpu_time::ProcessTime;
use crossbeam_channel::{Receiver, Sender, unbounded};
use bdd_sat_solver::{get_glucose_solution_no_malloc, GlucoseWrapper, parse_dimacs_and_add_clause_to_glucose, init_glucose_solver, run_glucose_parallel, print_incremental_stats};
use bdd_sat_solver::expr::bool_expr::Expr;
use bdd_sat_solver::parallel::clause_database::ClauseDatabase;
use bdd_sat_solver::parser::parse::parse_dimacs;
use bdd_sat_solver::statistics::stats::Stats;
use bdd_sat_solver::variable_ordering::var_ordering::BddVarOrdering;

#[tokio::main]
async fn main() {

    let tests = fs::read_dir("./benchmarks/tests").unwrap();
    //let paths11 = fs::read_dir("./benchmarks/benchmarks2011").unwrap();
    //let paths12 = fs::read_dir("./benchmarks/benchmarks2012").unwrap();
    //let paths13 = fs::read_dir("./benchmarks/benchmarks2013").unwrap();

    let names =
        tests.filter_map(|entry| {
            entry.ok().and_then(|e|
                e.path().file_name()
                    .and_then(|n| n.to_str().map(|s| String::from("./benchmarks/tests/".to_owned() + s)))
            )
        }).collect::<Vec<String>>();

    let paths = vec![names];

    for path in paths {
        let mut solved_with = 0;
        let mut solved_without = 0;
        let mut timedout_with = 0;
        let mut timedout_without = 0;

        let mut tasks = HashMap::new();
        for entry in path {
            tasks.insert(1, tokio::spawn(tokio::time::timeout(Duration::from_secs(1000), glucose_with_bdd(entry.clone()))));
            tasks.insert(0, tokio::spawn(tokio::time::timeout(Duration::from_secs(1000), glucose_without_bdd(entry.clone()))));
        }

        for (op, task) in tasks {
            if let Ok(succ) = task.await {
                if let Err(_) = succ {
                    if op == 1 { timedout_with += 1; } else { timedout_without += 1; }
                } else {
                    if op == 1 { solved_with += 1; } else { solved_without += 1; }
                }
            }
        }
        println!("Solved with:{:?}", solved_with);
        println!("Solved without:{:?}", solved_without);
        println!("Timed out with:{:?}", timedout_with);
        println!("Timed out without:{:?}", timedout_without);
    }
}

async fn glucose_with_bdd(path: String) {
    let (sender1, receiver1): (Sender<()>, Receiver<()>) = unbounded();
    let (sender2, receiver2) = (sender1.clone(), receiver1.clone());
    let (sender3, receiver3) = (sender2.clone(), receiver2.clone());

    // initate the statistics
    let mut stats = Stats::new();
    let mut stats_glucose = Stats::new();

    // initialize the expressions for the bdd
    let started = Instant::now();
    let start = ProcessTime::try_now().expect("Getting process time failed");

    // create the Dimacs instance
    let dimacs = parse_dimacs(&path);

    // create the vector of the parsed expressions
    let mut parsed_expr = Expr::parse_clauses(&dimacs.clauses);

    stats.parsing_time_bdd_world = started.elapsed();
    stats.parsing_time_bdd_cpu = start.try_elapsed().expect("Getting process time failed");

    // build the variable ordering
    let var_ordering = BddVarOrdering::new(&dimacs);

    // initiate the clause database
    let mut clause_database = ClauseDatabase::new();

    // initialize glucose
    let started = Instant::now();
    let start = ProcessTime::try_now().expect("Getting process time failed");

    let solver = init_glucose_solver();
    let nb_v = parse_dimacs_and_add_clause_to_glucose(path, solver);

    stats.parsing_time_glucose_world = started.elapsed();
    stats.parsing_time_glucose_cpu = start.try_elapsed().expect("Getting process time failed");

    println!("Glucose and Bdd initiated!");

    // pack glucose in a wrapper
    let solver_wrapper = GlucoseWrapper::new(solver);

    let (ret, _bdd) = rayon::join(|| run_glucose_parallel(solver_wrapper, sender1,sender2, sender3, &mut stats_glucose),
                                  || var_ordering.parallel_build(&mut parsed_expr, &mut clause_database, 0, solver_wrapper, &mut stats, receiver1,receiver2,receiver3));

    stats.solving_time_glucose_world = stats_glucose.solving_time_glucose_world;
    stats.solving_time_glucose_cpu = stats_glucose.solving_time_glucose_cpu;

    //let conflicts = get_conflicts_from_glucose(solver);
    //println!("{:?}", conflicts);

    match ret {
        0 => {
            println!("SAT");
            let mut sol = Vec::with_capacity(nb_v);
            get_glucose_solution_no_malloc(solver, &mut sol, nb_v);
        },
        _ => println!("UNSAT"),
    }

    println!("{:?}", stats);
    println!("{:?}", print_incremental_stats(solver));
}

async fn glucose_without_bdd(path: String) {

    //stats
    let mut stats = Stats::new();
    let mut stats_glucose = Stats::new();
    //dummy channel
    let (sender1, _receiver1): (Sender<()>, Receiver<()>) = unbounded();
    let (sender2, _receiver2): (Sender<()>, Receiver<()>) = unbounded();
    let (sender3, _receiver3): (Sender<()>, Receiver<()>) = unbounded();

    // initialize glucose
    let started = Instant::now();
    let start = ProcessTime::try_now().expect("Getting process time failed");

    let solver = init_glucose_solver();
    let nb_v = parse_dimacs_and_add_clause_to_glucose(path, solver);

    // pack glucose in a wrapper
    let solver_wrapper = GlucoseWrapper::new(solver);

    stats.parsing_time_glucose_world = started.elapsed();
    let cpu_time: Duration = start.try_elapsed().expect("Getting process time failed");
    stats.parsing_time_glucose_cpu = cpu_time;

    let ret = run_glucose_parallel(solver_wrapper, sender1,sender2,sender3, &mut stats_glucose);
    match ret {
        0 => {
            let mut sol = Vec::with_capacity(nb_v);
            get_glucose_solution_no_malloc(solver, &mut sol, nb_v);
        },
        _ => println!("UNSAT"),
    }

    //let conflicts = get_conflicts_from_glucose(solver);
    //println!("{:?}", conflicts);

    stats.solving_time_glucose_world = stats_glucose.solving_time_glucose_world;
    stats.solving_time_glucose_cpu = stats_glucose.solving_time_glucose_cpu;

    println!("Stats: {:?}", stats);
    println!("{:?}", print_incremental_stats(solver));
}