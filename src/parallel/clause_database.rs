// Overall, there are three reasons why a clause offered by a
// core solver can get discarded. One is that it was duplicate
// or wrongly considered to be duplicate due to the probabilistic
// nature of Bloom filters. Second is that another core solver was adding
// its clause to the data structure for global export at the same time.
// The last reason is that it did not fit into the fixed size message
// sent to the other MPI processes. Although important learned clauses
// might get lost, we believe that this relaxed approach is still beneficial
// since it allows a simpler and more efficient implementation of clause sharing.

extern crate bit_set;

use std::collections::hash_map::RandomState;
use bit_set::BitSet;
use bloom_filters::{BloomFilter, ClassicBloomFilter, DefaultBuildHashKernels};
use rand::random;
use crate::{add_assumptions_to_glucose_solver, add_incoming_clause_to_clauses_vec, GlucoseWrapper};
use crate::statistics::stats::Stats;

pub struct ClauseDatabase {
    pub global_filter_bloom: ClassicBloomFilter<DefaultBuildHashKernels<RandomState>>,
    pub local_filter_bloom: ClassicBloomFilter<DefaultBuildHashKernels<RandomState>>,
    pub global_filter: ClauseFilter,
    pub local_filter: ClauseFilter,
    pub global_filter_vec: Vec<Vec<i32>>,
    pub local_filter_vec: Vec<Vec<i32>>
}

impl ClauseDatabase {

    pub fn new() -> ClauseDatabase {
        // classic bloom filter
        let filter_local = ClassicBloomFilter::new(100, 0.03, DefaultBuildHashKernels::new(random(), RandomState::new()));
        let filter_global = ClassicBloomFilter::new(100, 0.03, DefaultBuildHashKernels::new(random(), RandomState::new()));

        ClauseDatabase {
            global_filter_bloom: filter_global,
            local_filter_bloom: filter_local,
            global_filter: ClauseFilter::new(),
            local_filter: ClauseFilter::new(),
            global_filter_vec: Vec::new(),
            local_filter_vec: Vec::new()
        }
    }

    pub fn get_next_incoming_clause(&mut self, clause: Vec<i32>) -> Result<Vec<i32>, &str> {
        if self.global_filter.register_clause(&clause) {
            if self.local_filter.register_clause(&clause) {
                return Ok(clause);
            }
        }
        Err("Clause didn't pass the filters")
    }

    pub fn get_next_incoming_clause_bloom(&mut self, clause: Vec<i32>) -> Result<Vec<i32>, &str> {
        if !self.bloom_filter_global_contains(&clause) {
            self.insert_to_bloom_filter_global(&clause);
            if !self.bloom_filter_local_contains(&clause) {
                self.insert_to_bloom_filter_local(&clause);
                return Ok(clause);
            }
        }
        Err("Clause didn't pass the filters")
    }

    pub fn get_next_incoming_clause_vec(&mut self, clause: Vec<i32>) -> Result<Vec<i32>, &str> {
        if !self.global_filter_vec.contains(&clause) {
            self.global_filter_vec.push(clause.clone());
            if !self.local_filter_vec.contains(&clause) {
                self.local_filter_vec.push(clause.clone());
                return Ok(clause.clone());
            }
        }
        Err("Clause didn't pass the filters")
    }

    fn insert_to_bloom_filter_local(&mut self, clause: &Vec<i32>) {
        clause.iter().for_each(|i| self.local_filter_bloom.insert(i));
    }

    fn insert_to_bloom_filter_global(&mut self, clause: &Vec<i32>) {
        clause.iter().for_each(|i| self.global_filter_bloom.insert(i));
    }

    fn bloom_filter_local_contains(&mut self, clause: &Vec<i32>) -> bool {
        for i in clause.iter() {
            if !self.local_filter_bloom.contains(i) {
                return false;
            }
        }
        true
    }

    fn bloom_filter_global_contains(&mut self, clause: &Vec<i32>) -> bool {
        for i in clause.iter() {
            if !self.global_filter_bloom.contains(i) {
                return false;
            }
        }
        true
    }

    pub fn reset_bloom_filter_global(&mut self) {
        self.global_filter_bloom.reset();
    }

    pub fn reset_bloom_filter_local(&mut self) {
        self.local_filter_bloom.reset();
    }

    pub fn reset_filter_global(&mut self) {
        self.global_filter.clear()
    }

    pub fn reset_filter_local(&mut self) {
        self.local_filter.clear();
    }

    /// This method receives a sharing manager and a clause database, which were
    /// initialized when a solver thread was initialized. Every solver has a rank.
    /// The method receives the clause as a message from the mpi together with its status.
    /// It sends the clause to the database where it is filtered and it receives back
    /// the clause or an error that the clause did not pass the databases' filters.
    /// After that the learned clause has to be sent back to the solvers but it can't
    /// be sent back to the solver it came from.
    pub fn send(&mut self, clause_input: Vec<i32>, solver_wrapper: GlucoseWrapper, stats: &mut Stats) {
        stats.add_sent_bdd();

        // both need to be registered to the clause database
        if let Ok(learned_clause) = self.get_next_incoming_clause_bloom(clause_input) {
            // the clause passed the filters so send it to glucose
            let solver = solver_wrapper.0;
            // add the clause to glucoses receive_tmp so that glucose catches it from there
            add_incoming_clause_to_clauses_vec(solver, learned_clause);
            stats.add_received_glucose();
        }
    }

    pub fn receive(&mut self, _solver_wrapper: GlucoseWrapper, _stats: &mut Stats) -> Option<Vec<i32>> {
        // basically we assigned the contents of learnt_clause of glucose to add_tmp_send
        // so now we need to take the contents of add_tmp_send
        /*let solver = solver_wrapper.0;
        if let Some(received_glucose) = get_exported_clause_from_glucose(solver) {
            stats.add_sent_glucose();

            // both need to be registered to the clause database
            if let Ok(learned_clause) = self.get_next_incoming_clause_bloom(received_glucose) {
                // the clause passed the filters so send it to the bdd
                stats.add_received_bdd();
                return Some(learned_clause);
            }
        }*/
        None
    }

    pub fn send_assumptions(&mut self, clause_input: Vec<i32>, solver_wrapper: GlucoseWrapper, stats: &mut Stats) {

        // get the clause received
        //let clause_received_core_1 = self.receiver_global_from_bdd.try_recv().context("Core 1 channel has hung up")?;
        stats.add_sent_bdd();

        if let Ok(learned_clause) = self.get_next_incoming_clause_bloom(clause_input) {
            // the clause passed the filters
            let solver = solver_wrapper.0;
            // send assumptions to glucose
            add_assumptions_to_glucose_solver(solver, learned_clause);
            stats.add_received_glucose();
        }
    }
}

/// self implemented bloom filter
const PRIMES: [i64;12] = [2038072819, 2038073287, 2038073761, 2038074317,
    2038072823, 2038073321, 2038073767, 2038074319,
    2038072847, 2038073341, 2038073789, 2038074329];
const NUM_PRIMES: i64 = 12;
const NUM_BITS: usize = 26843543; // 3,2MB

pub struct ClauseFilter(BitSet);

impl ClauseFilter {
    fn new() -> ClauseFilter {
        let s = BitSet::new();
        ClauseFilter(s)
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn has(&self, h: usize) -> bool {
        self.0.contains(h)
    }

    fn set(&mut self, h: usize) -> bool {
        self.0.insert(h)
    }

    fn commutative_hash_function(&self, clause: &Vec<i32>, which: i64) -> usize {
        let mut res = 0;
        for lit in clause {
            let lit_i64 = *lit as i64;
            res ^= lit_i64 * PRIMES[((which * lit_i64) % NUM_PRIMES).abs() as usize];
        }
        res as usize % NUM_BITS
    }

    fn register_clause(&mut self, clause: &Vec<i32>) -> bool {
        // unit clauses always get in
        if clause.len() == 1 {
            return true;
        }
        let h1 = self.commutative_hash_function(clause, 1);
        let h2 = self.commutative_hash_function(clause, 2);
        let h3 = self.commutative_hash_function(clause, 3);
        let h4 = self.commutative_hash_function(clause, 4);

        return if self.has(h1) && self.has(h2) && self.has(h3) && self.has(h4) {
            false
        } else {
            self.set(h1);
            self.set(h2);
            self.set(h3);
            self.set(h4);
            true
        }
    }
}
