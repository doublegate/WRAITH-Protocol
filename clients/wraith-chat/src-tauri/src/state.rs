// Application State Management

use crate::crypto::DoubleRatchet;
use crate::database::Database;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Global application state
pub struct AppState {
    /// Database connection
    pub db: Mutex<Database>,

    /// Double Ratchet states for each peer
    pub ratchets: Mutex<HashMap<String, DoubleRatchet>>,

    /// Local peer ID
    pub local_peer_id: Mutex<String>,
    // WRAITH node (TODO: Add wraith_core::Node when integrated)
    // pub node: Mutex<Option<Arc<Node>>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database) -> Self {
        Self {
            db: Mutex::new(db),
            ratchets: Mutex::new(HashMap::new()),
            local_peer_id: Mutex::new(String::new()),
        }
    }
}
