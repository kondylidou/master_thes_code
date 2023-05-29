use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct Stats {
    pub parsing_time_glucose_world: Duration,
    pub solving_time_glucose_world: Duration,
    pub parsing_time_glucose_cpu: Duration,
    pub solving_time_glucose_cpu: Duration,
    pub parsing_time_bdd_world: Duration,
    pub parsing_time_bdd_cpu: Duration,
    sent_clauses_glucose: u64,
    received_clauses_glucose: u64,
    sent_clauses_bdd: u64,
    received_clauses_bdd: u64,
    t_send_learned_clauses: Vec<Duration>,
    t_approx: Vec<Duration>,
    bdd_size: Vec<usize>,
}

impl Stats {

    pub fn new() -> Stats {
        Stats {
            parsing_time_glucose_world: Default::default(),
            solving_time_glucose_world: Default::default(),
            parsing_time_glucose_cpu: Default::default(),
            solving_time_glucose_cpu: Default::default(),
            parsing_time_bdd_world: Default::default(),
            parsing_time_bdd_cpu: Default::default(),
            sent_clauses_glucose: 0,
            received_clauses_glucose: 0,
            sent_clauses_bdd: 0,
            received_clauses_bdd: 0,
            t_send_learned_clauses: Vec::new(),
            t_approx: Vec::new(),
            bdd_size: Vec::new(),
        }
    }

    fn fields(&self) -> HashMap<String, Duration> {
        let mut fields = HashMap::new();

        fields.insert("Parsing time glucose world".to_string(), self.parsing_time_glucose_world);
        fields.insert("Solving time glucose world".to_string(), self.solving_time_glucose_world);
        fields.insert("Parsing time glucose cpu".to_string(), self.parsing_time_glucose_cpu);
        fields.insert("Solving time glucose cpu".to_string(), self.solving_time_glucose_cpu);
        fields.insert("Parsing time bdd world".to_string(), self.parsing_time_bdd_world);
        fields.insert("Parsing time bdd cpu".to_string(), self.parsing_time_bdd_cpu);
        fields.insert("Average time to send learned clauses".to_string(), self.t_send());
        fields.insert("Average time to approximate bdd".to_string(), self.t_approx());
        fields
    }

    fn plots(&self) -> HashMap<String, u64> {
        let mut plots = HashMap::new();

        plots.insert("Clauses number sent from glucose".to_string(), self.sent_clauses_glucose);
        plots.insert("Clauses number sent from bdd".to_string(), self.sent_clauses_bdd);
        plots.insert("Clauses number received at glucose".to_string(), self.received_clauses_glucose);
        plots.insert("Clauses number received at bdd".to_string(), self.received_clauses_bdd);
        plots
    }

    fn t_send(&self) -> Duration {
        let mut sum = Duration::new(0, 0);
        match self {
            Stats { t_send_learned_clauses, .. } => {
                if t_send_learned_clauses.to_vec().is_empty() { sum }
                else {
                    let avg: Duration;
                    for t in &t_send_learned_clauses.to_vec() { sum = sum + *t; }
                    avg = sum / t_send_learned_clauses.to_vec().len() as u32;
                    avg
                }
            }
        }
    }

    pub fn add_t_send(&mut self, dur: Duration) { self.t_send_learned_clauses.push(dur); }
    pub fn add_t_approx(&mut self, dur: Duration) { self.t_approx.push(dur); }

    pub fn add_bdd_size(&mut self, size: usize) { self.bdd_size.push(size); }

    pub fn add_received_glucose(&mut self) { self.received_clauses_glucose += 1; }
    pub fn add_received_bdd(&mut self) { self.received_clauses_bdd += 1; }
    pub fn add_sent_glucose(&mut self) { self.sent_clauses_glucose += 1; }
    pub fn add_sent_bdd(&mut self) { self.sent_clauses_bdd += 1; }


    fn t_approx(&self) -> Duration {
        let mut sum = Duration::new(0, 0);
        match self {
            Stats { t_approx, .. } => {
                if t_approx.to_vec().is_empty() { sum }
                else {
                    let avg: Duration;
                    for t in &t_approx.to_vec() { sum = sum + *t; }
                    avg = sum / t_approx.to_vec().len() as u32;
                    avg
                }
            }
        }
    }
}

impl std::fmt::Debug for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, val) in self.fields().iter() {
            write!(f, "{}: {:?}\n", key, val)?;
        }
        for (key, val) in self.plots().iter() {
            write!(f, "{}: {:?}\n", key, val)?;
        }
        write!(f, "bdd size: {:?}\n", self.bdd_size)?;
        Ok(())
    }
}
