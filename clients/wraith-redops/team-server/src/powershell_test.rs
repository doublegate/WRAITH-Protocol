#[cfg(test)]
mod tests {
    use crate::services::powershell::{JobStatus, PowerShellManager};
    use uuid::Uuid;

    #[test]
    fn test_powershell_manager_profiles() {
        let manager = PowerShellManager::new();
        let implant_id = Uuid::new_v4();

        assert_eq!(manager.get_profile(implant_id), None);

        let profile_script = "function Test-Func { Write-Host 'Hello' }";
        manager.set_profile(implant_id, profile_script);

        assert_eq!(
            manager.get_profile(implant_id),
            Some(profile_script.to_string())
        );
    }

    #[test]
    fn test_powershell_job_tracking() {
        let manager = PowerShellManager::new();
        let implant_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();

        // 1. Add Job via Manager
        manager.create_job(implant_id, job_id, "Get-Process");

        // 2. Verify Job state
        {
            let session = manager.get_or_create_session(implant_id);
            let job = session.jobs.get(&job_id).unwrap();
            assert_eq!(job.command, "Get-Process");
            assert_eq!(job.status, JobStatus::Pending);
            assert!(job.output_buffer.is_empty());
        }

        // 3. Update status (manual session access)
        {
            let session = manager.get_or_create_session(implant_id);
            session.update_job_status(job_id, JobStatus::Running);
        }

        // 4. Verify Status
        {
            let session = manager.get_or_create_session(implant_id);
            assert_eq!(
                session.jobs.get(&job_id).unwrap().status,
                JobStatus::Running
            );
        }

        // 5. Append output via Manager
        manager.append_output(job_id, b"Process 1\n");

        // 6. Verify Output
        {
            let session = manager.get_or_create_session(implant_id);
            assert_eq!(
                session.jobs.get(&job_id).unwrap().output_buffer,
                b"Process 1\n"
            );
        }

        // 7. Complete
        {
            let session = manager.get_or_create_session(implant_id);
            session.set_exit_code(job_id, 0);
            session.update_job_status(job_id, JobStatus::Completed);

            let completed_job = session.jobs.get(&job_id).unwrap();
            assert_eq!(completed_job.status, JobStatus::Completed);
            assert_eq!(completed_job.exit_code, Some(0));
            assert!(completed_job.completed_at.is_some());
        }
    }
}
