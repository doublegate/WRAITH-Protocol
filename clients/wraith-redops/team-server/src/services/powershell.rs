use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerShellJob {
    pub id: Uuid,
    pub command: String,
    pub status: JobStatus,
    pub output_buffer: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
}

pub struct PowerShellSession {
    pub profile: Option<String>,
    pub jobs: DashMap<Uuid, PowerShellJob>,
}

impl PowerShellSession {
    pub fn new() -> Self {
        Self {
            profile: None,
            jobs: DashMap::new(),
        }
    }

    pub fn add_job_with_id(&self, id: Uuid, command: &str) {
        let job = PowerShellJob {
            id,
            command: command.to_string(),
            status: JobStatus::Pending,
            output_buffer: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
            exit_code: None,
        };
        self.jobs.insert(id, job);
    }

    pub fn append_output(&self, job_id: Uuid, data: &[u8]) {
        if let Some(mut job) = self.jobs.get_mut(&job_id) {
            job.output_buffer.extend_from_slice(data);
        }
    }
}

pub struct PowerShellManager {
    pub sessions: DashMap<Uuid, PowerShellSession>,
    pub job_map: DashMap<Uuid, Uuid>,
}

impl PowerShellManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            job_map: DashMap::new(),
        }
    }

    pub fn get_or_create_session(
        &self,
        implant_id: Uuid,
    ) -> dashmap::mapref::one::RefMut<'_, Uuid, PowerShellSession> {
        self.sessions
            .entry(implant_id)
            .or_insert_with(PowerShellSession::new)
    }

    pub fn create_job(&self, implant_id: Uuid, job_id: Uuid, command: &str) {
        let session = self.get_or_create_session(implant_id);
        session.add_job_with_id(job_id, command);
        self.job_map.insert(job_id, implant_id);
    }

    pub fn append_output(&self, job_id: Uuid, data: &[u8]) {
        if let Some(implant_id) = self.job_map.get(&job_id)
            && let Some(session) = self.sessions.get_mut(&*implant_id)
        {
            session.append_output(job_id, data);
        }
    }

    pub fn set_profile(&self, implant_id: Uuid, profile: &str) {
        let mut session = self.get_or_create_session(implant_id);
        session.profile = Some(profile.to_string());
    }

    pub fn get_profile(&self, implant_id: Uuid) -> Option<String> {
        self.sessions
            .get(&implant_id)
            .and_then(|s| s.profile.clone())
    }
}

impl Default for PowerShellManager {
    fn default() -> Self {
        Self::new()
    }
}
