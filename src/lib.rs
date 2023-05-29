pub mod parser {
    pub mod parse;
}
pub mod variable_ordering {
    pub mod var_ordering_builder;
    pub mod var_ordering;
}
pub mod bdd;
pub mod bdd_util;
pub mod approx;
mod clause_gen;

pub mod statistics {
    pub mod stats;
}

pub mod expr { pub mod bool_expr; }
pub mod parallel { pub mod clause_database; }
pub mod sharing { pub mod sharing_manager; }

pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/glucose_bindings.rs"));
}

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use cpu_time::ProcessTime;
use crossbeam_channel::Sender;
use bindings::*;
use crate::statistics::stats::Stats;

#[derive(Clone, Copy)]
pub struct GlucoseWrapper(pub *mut CGlucose);
impl GlucoseWrapper {
    pub fn new(solver: *mut CGlucose) -> GlucoseWrapper {
        GlucoseWrapper(solver)
    }
}
unsafe impl Send for GlucoseWrapper {}
unsafe impl Sync for GlucoseWrapper {}


use bindings::CGlucose;
use bindings::cglucose_init;
use bindings::cglucose_assume;
use bindings::cglucose_add_to_clause;
use bindings::cglucose_commit_clause;
use bindings::cglucose_clean_clause;
use bindings::cglucose_solve;
use bindings::cglucose_val;
use bindings::cglucose_solver_nodes;
use bindings::cglucose_nb_learnt;
use bindings::cglucose_set_random_seed;

pub fn init_glucose_solver() -> *mut CGlucose {
    let s : *mut CGlucose =  unsafe { cglucose_init() };
    s
}

pub fn add_assumptions_to_glucose_solver(s : *mut CGlucose, assumptions : Vec<i32>){
    unsafe {
        for i in assumptions{
            cglucose_assume(s, i);
        }
    }
}

pub fn run_glucose(s : *mut CGlucose) -> i32 {
    let ret = unsafe { cglucose_solve(s) };
    ret
}

pub fn add_clause_to_glucose_solver(s : *mut CGlucose, given : Vec<i32>){
    unsafe {
        cglucose_clean_clause(s);
        for i in given{
            cglucose_add_to_clause(s, i);
        }
        cglucose_commit_clause(s);
    }
}

pub fn get_glucose_solver_stats(s : *mut CGlucose) -> u64 {
    let nodes = unsafe { cglucose_solver_nodes(s) };
    nodes
}

/// Gets a solution from Glucose solver while using the given nb_vars
/// to allocate a new Rust solution vec to write and return.
pub fn get_glucose_solution(s : *mut CGlucose, nb_vars : usize) -> Vec<i32>{
    let mut model : Vec<i32> = Vec::with_capacity(nb_vars);
    for i in 1..nb_vars+1{
        let b = unsafe { cglucose_val(s, (i-1) as i32)};
        // #define l_True  (Glucose::lbool((uint8_t)0))
        // #define l_False (Glucose::lbool((uint8_t)1))
        // #define l_Undef (Glucose::lbool((uint8_t)2))
        if b == 0 {
            model.push(i as i32);
        } else if b == 1 {
            model.push(-(i as i32));
        } else if b == 2 {
            panic!("Model has an undefined value!");
        }
    }
    model
}

/// Gets a solution from Glucose solver and writes on to the given Vector.
/// No memory allocation is done here unless the given model has a smaller capacity then the given nb_vars.
pub fn get_glucose_solution_no_malloc(s : *mut CGlucose, model : &mut Vec<i32>, nb_vars : usize){
    model.clear();
    for i in 1..nb_vars+1{
        let b = unsafe { cglucose_val(s, (i-1) as i32)};
        // #define l_True  (Glucose::lbool((uint8_t)0))
        // #define l_False (Glucose::lbool((uint8_t)1))
        // #define l_Undef (Glucose::lbool((uint8_t)2))
        if b == 0 {
            model.push(i as i32);
        } else if b == 1 {
            model.push(-(i as i32));
        } else if b == 2 {
            panic!("Model has an undefined value!");
        }
    }
}

pub fn set_glucose_rnd_seed(s : *mut CGlucose, seed: f64){
    unsafe { cglucose_set_random_seed(s, seed) };
}

pub fn get_glucose_solver_nb_learnt(s : *mut CGlucose) -> u64 {
    return unsafe { cglucose_nb_learnt(s) };
}

pub fn print_incremental_stats(s : *mut CGlucose) {
    unsafe { cglucose_print_incremental_stats(s) };
}

pub fn run_glucose_parallel(solver_wrapper : GlucoseWrapper, sender1: Sender<()>,sender2: Sender<()>,sender3: Sender<()>, stats_glucose: &mut Stats) -> i32 {
    let started = Instant::now();
    let start = ProcessTime::try_now().expect("Getting process time failed");

    let s = solver_wrapper.0;

    let ret = unsafe { cglucose_solve(s) };

    println!("Glucose terminated.");
    println!(" ");
    stats_glucose.solving_time_glucose_world = started.elapsed();
    stats_glucose.solving_time_glucose_cpu = start.try_elapsed().expect("Getting process time failed");

    // inform the other thread to terminate
    sender1.send(()).expect("Channel was disconnected");
    sender2.send(()).expect("Channel was disconnected");
    sender3.send(()).expect("Channel was disconnected");
    ret
}

pub fn add_incoming_clause_to_clauses_vec(s : *mut CGlucose, given : Vec<i32>){
    unsafe {
        cglucose_clean_clause_receive(s);
        for i in given{
            cglucose_add_to_clause_receive(s, i as i32);
        }
        cglucose_commit_incoming_clause(s);
    }
}

pub fn get_glucose_val(s : *mut CGlucose, i: i32) -> i32 {
    unsafe {
        cglucose_val(s, (i-1) as i32)
    }
}
/*
pub fn get_exported_clause_size(s : *mut CGlucose) -> i32 {
    return unsafe {cglucose_get_n_tmp_send(s)}
}

pub fn get_exported_lit_at(s : *mut CGlucose, pos: i32) -> i32 {
    return unsafe {cglucose_get_tmp_send_lit_at(s, pos)}
}

pub fn get_exported_clause_from_glucose(s : *mut CGlucose) -> Option<Vec<i32>> {
    let size = get_exported_clause_size(s);
    if size == 0 {
        None
    } else {
        let mut exported_clause = Vec::new();

        let mut pos = 0;
        while pos < size {
            let lit = get_exported_lit_at(s, pos);
            exported_clause.push(lit);
            pos += 1;
        }
        unsafe { cglucose_clean_clause_send(s); }
        Some(exported_clause)
    }
}*/

/*
pub fn get_conflicts_vec_size(s : *mut CGlucose) -> i32 {
    return unsafe {cglucose_get_add_conflicts_size(s)}
}

pub fn get_conflicts_at(s : *mut CGlucose, pos: i32) -> i32 {
    return unsafe {cglucose_get_conflicts_at(s, pos)}
}

pub fn get_conflicts_from_glucose(s : *mut CGlucose) -> Vec<i32> {
    let size = get_conflicts_vec_size(s);
    let mut conflicts = Vec::new();

    let mut pos = 0;
    while pos < size {
        let conflict = get_conflicts_at(s, pos);
        conflicts.push(conflict);
        pos += 1;
    }
    conflicts
}
*/

pub fn parse_dimacs_and_add_clause_to_glucose(path: String, solver : *mut CGlucose) -> usize {
    let input = File::open(path).unwrap();
    let buffered = BufReader::new(input);
    let mut _nb_c: usize;
    let mut nb_v: usize = 0;
    for line in buffered.lines() {
        let l = line.unwrap();
        if l.contains("p") && l.contains("cnf") {
            let i : Vec<&str> = l.split_whitespace().collect();
            nb_v = i[2].to_string().parse().unwrap();
            _nb_c = i[3].to_string().parse().unwrap();
        }
        else if l.is_empty() || l.contains("c") {
            continue;
        }  else {
            let iter = l.split_whitespace();
            let mut v_clause : Vec<i32> = vec![];
            'iter: for i in iter {
                let int: i32 = i.parse().unwrap();
                if int == 0 {
                    break 'iter;
                }
                v_clause.push(int);
            }
            add_clause_to_glucose_solver(solver, v_clause);
        }
    }
    nb_v
}