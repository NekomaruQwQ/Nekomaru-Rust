use std::io;
use std::sync::{Mutex, OnceLock};

/// Retains the process-group handle until Windows closes it during process teardown.
static KILL_CHILDREN_ON_EXIT_JOB: OnceLock<win32job::Job> = OnceLock::new();

/// Serializes retryable initialization before the process-group handle exists.
static KILL_CHILDREN_ON_EXIT_INITIALIZATION: Mutex<()> = Mutex::new(());

/// Ensures child processes spawned after this call exit with the current process.
///
/// The first successful call assigns the current process to a persistent Windows
/// process group. Later calls are no-ops. Child processes that already exist when
/// this function is called are not affected, and explicitly detached children may
/// escape the process group.
///
/// # Errors
///
/// Returns an error if the process group cannot be created, configured, or assigned
/// to the current process. A later call retries initialization after an error.
pub fn kill_children_on_exit() -> io::Result<()> {
    use win32job::ExtendedLimitInfo;
    use win32job::Job;

    if KILL_CHILDREN_ON_EXIT_JOB.get().is_some() {
        return Ok(());
    }

    let _initialization_guard =
        KILL_CHILDREN_ON_EXIT_INITIALIZATION
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    if KILL_CHILDREN_ON_EXIT_JOB.get().is_some() {
        return Ok(());
    }

    let mut limit_info = ExtendedLimitInfo::new();
    limit_info.limit_kill_on_job_close();

    let job_object =
        Job::create_with_limit_info(&limit_info)
            .map_err(io::Error::from)?;

    // CONTEXT: Keep assignment as the final fallible operation because dropping
    // this last handle after assignment would terminate the current process.
    job_object
        .assign_current_process()
        .map_err(io::Error::from)?;

    if let Err(job_object) = KILL_CHILDREN_ON_EXIT_JOB.set(job_object) {
        // CONTEXT: The initialization lock makes this unreachable, but leaking the
        // redundant handle is safer than terminating the current process on drop.
        let _leaked_handle = job_object.into_handle();
    }
    Ok(())
}
