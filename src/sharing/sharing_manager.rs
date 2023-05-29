/*
pub struct GlobalSharingManager {
    sender_global_to_bdd: Sender<Vec<i32>>,
    sender_global_to_glucose: Sender<Vec<i32>>,
    receiver_global_from_bdd: Receiver<Vec<i32>>,
    receiver_global_from_glucose: Receiver<Vec<i32>>,
    pub clause_database: ClauseDatabase,
}

impl GlobalSharingManager {
    pub fn new(
        sender_global_to_bdd: Sender<Vec<i32>>,
        sender_global_to_glucose: Sender<Vec<i32>>,
        receiver_global_from_bdd: Receiver<Vec<i32>>,
        receiver_global_from_glucose: Receiver<Vec<i32>>,
        clause_database: ClauseDatabase,
    ) -> GlobalSharingManager {
        GlobalSharingManager {
            sender_global_to_bdd,
            sender_global_to_glucose,
            receiver_global_from_bdd,
            receiver_global_from_glucose,
            clause_database,
        }
    }
}

unsafe impl Send for GlobalSharingManager {}
unsafe impl Sync for GlobalSharingManager {}

pub struct SharingManager {
    pub sender: Sender<Vec<i32>>,
    pub receiver: Receiver<Vec<i32>>,
    pub solver_id: i32,
}

impl SharingManager {
    pub fn new(
        sender: Sender<Vec<i32>>,
        receiver: Receiver<Vec<i32>>,
        solver_id: i32,
    ) -> SharingManager {
        SharingManager {
            sender,
            receiver,
            solver_id,
        }
    }
}

unsafe impl Send for SharingManager {}
unsafe impl Sync for SharingManager {}
 */