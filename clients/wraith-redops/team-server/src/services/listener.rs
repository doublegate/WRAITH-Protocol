use crate::database::Database;
use crate::governance::GovernanceEngine;
use crate::listeners;
use crate::services::session::SessionManager;
use crate::wraith::redops::Event;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::AbortHandle;
use wraith_crypto::noise::NoiseKeypair;

pub struct ListenerManager {
    // Map listener_id -> AbortHandle
    active_listeners: DashMap<String, AbortHandle>,
    db: Arc<Database>,
    governance: Arc<GovernanceEngine>,
    sessions: Arc<SessionManager>,
    static_key: Arc<NoiseKeypair>,
    event_tx: broadcast::Sender<Event>,
}

impl ListenerManager {
    pub fn new(
        db: Arc<Database>,
        governance: Arc<GovernanceEngine>,
        sessions: Arc<SessionManager>,
        static_key: Arc<NoiseKeypair>,
        event_tx: broadcast::Sender<Event>,
    ) -> Self {
        Self {
            active_listeners: DashMap::new(),
            db,
            governance,
            sessions,
            static_key,
            event_tx,
        }
    }

    pub async fn start_listener(
        &self,
        id: &str,
        type_: &str,
        bind_addr: &str,
        port: u16,
    ) -> Result<(), String> {
        if self.active_listeners.contains_key(id) {
            return Err("Listener already active".to_string());
        }

        let db = self.db.clone();
        let gov = self.governance.clone();
        let sess = self.sessions.clone();
        let key = (*self.static_key).clone();
        let tx = self.event_tx.clone();

        let handle = match type_ {
            "http" => tokio::spawn(async move {
                listeners::http::start_http_listener(db, port, tx, gov, key, sess).await;
            }),
            "udp" => tokio::spawn(async move {
                listeners::udp::start_udp_listener(db, port, tx, gov, key, sess).await;
            }),
            "dns" => tokio::spawn(async move {
                listeners::dns::start_dns_listener(db, port, tx, gov, key, sess).await;
            }),
            "smb" => tokio::spawn(async move {
                listeners::smb::start_smb_listener(db, port, tx, gov, key, sess).await;
            }),
            _ => return Err("Unknown listener type".to_string()),
        };

        self.active_listeners
            .insert(id.to_string(), handle.abort_handle());
        tracing::info!(
            "Started listener {} type {} on {}:{}",
            id,
            type_,
            bind_addr,
            port
        );
        Ok(())
    }

    pub async fn stop_listener(&self, id: &str) -> Result<(), String> {
        if let Some((_, handle)) = self.active_listeners.remove(id) {
            handle.abort();
            tracing::info!("Stopped listener {}", id);
            Ok(())
        } else {
            Err("Listener not active".to_string())
        }
    }
}
