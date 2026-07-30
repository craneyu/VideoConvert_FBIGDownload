//! The shared budget for CPU-bound work.
//!
//! Two pipelines re-encode video: the step that runs after a download, and the
//! transcoding tab. Neither can observe what the other is doing, and libx264
//! already scales across every available core — so two concurrent re-encodes halve
//! each other rather than adding throughput. A per-pipeline limit cannot express
//! that; only a budget both pipelines draw from can.
//!
//! Deliberately narrow: this owns the CPU budget and nothing else. How many
//! downloads run their *network* phase at once stays with the download queue in
//! the frontend, which already decides how many download commands to issue.

use std::sync::Arc;
use tokio::sync::{OnceCell, OwnedSemaphorePermit, Semaphore};

/// A held CPU permit. The permit returns to the pool when this is dropped, so
/// callers keep it alive for exactly as long as the encode runs.
#[derive(Debug)]
pub struct CpuPermit(#[allow(dead_code)] OwnedSemaphorePermit);

/// How many re-encodes are allowed to run at once, across both pipelines.
pub struct CpuBudget {
    semaphore: Arc<Semaphore>,
}

impl CpuBudget {
    /// Build a budget with `limit` permits.
    ///
    /// A limit of zero would stop every encode with no way to tell, so it is
    /// raised to one. The settings layer already rejects zero; this is the second
    /// line of defence for a caller that computes the limit some other way.
    pub fn new(limit: u32) -> Self {
        let limit = limit.max(1) as usize;
        Self {
            semaphore: Arc::new(Semaphore::new(limit)),
        }
    }

    /// Wait for a permit.
    ///
    /// Awaiting here does not occupy a runtime worker, which is why the blocking
    /// encode work is kept in `spawn_blocking` and the wait is not.
    ///
    /// `Err` means the pool was closed and no permit will ever be granted. The
    /// caller must surface it rather than proceed — starting an encode without a
    /// permit would defeat the budget silently.
    pub async fn acquire(&self) -> Result<CpuPermit, String> {
        Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map(CpuPermit)
            .map_err(|_| "CPU budget is closed; refusing to start an encode".to_string())
    }

    /// Stop granting permits. Only used to prove the failure path in tests.
    #[cfg(test)]
    fn close(&self) {
        self.semaphore.close();
    }

    /// How many permits are currently free.
    #[cfg(test)]
    fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// Run blocking work off the async runtime.
///
/// Every long step below reads a child process's output line by line and waits for
/// it to exit. Doing that directly in an `async` command occupies a runtime worker
/// for the whole download, so unrelated IPC — reading settings, querying history —
/// stalls behind it.
///
/// Dropping the `async` keyword does not fix this. A command without it runs
/// synchronously on the IPC thread, which is worse; and marking a synchronous
/// function `#[tauri::command(async)]` still hands the call to
/// `async_runtime::spawn`, despite the "sync_threadpool" label the macro records.
/// Only an explicit `spawn_blocking` leaves the worker pool.
///
/// A panic inside `work` becomes an error string rather than taking the runtime
/// down with it.
pub async fn run_blocking<F, R>(phase: &str, work: F) -> Result<R, String>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("{} terminated unexpectedly: {}", phase, e))
}

/// The application-wide handle to the CPU budget, held in Tauri's managed state.
///
/// The pool is built the first time a permit is needed rather than at startup: the
/// database connection is established when the frontend loads it, and the
/// migrations run at that point too, so during setup the settings table may not be
/// readable — or may not exist. Reading it then would silently use the default on
/// every launch and discard whatever the user configured.
///
/// Once built the capacity never changes, which is what makes "changing this
/// setting takes effect on the next launch" true.
#[derive(Default)]
pub struct SharedCpuBudget {
    cell: OnceCell<CpuBudget>,
}

impl SharedCpuBudget {
    /// Wait for a CPU permit, building the pool from `limit` if this is the first
    /// call.
    ///
    /// **`limit` is honoured only on the first call.** Every later call passes a
    /// value that is ignored, because resizing a live pool would require deciding
    /// what happens to permits already handed out. Callers must not treat this as
    /// a way to change the limit.
    pub async fn acquire(&self, limit: u32) -> Result<CpuPermit, String> {
        self.cell
            .get_or_init(|| async { CpuBudget::new(limit) })
            .await
            .acquire()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::async_runtime::block_on;

    #[test]
    fn a_permit_is_granted_while_the_budget_has_room() {
        let budget = CpuBudget::new(1);
        block_on(async {
            let permit = budget.acquire().await.expect("first acquire should succeed");
            assert_eq!(budget.available(), 0, "the only permit is now held");
            drop(permit);
        });
        assert_eq!(budget.available(), 1, "dropping the guard returns the permit");
    }

    #[test]
    fn a_second_acquire_waits_until_the_first_permit_is_released() {
        // The behaviour the whole module exists for: with a limit of 1, a second
        // encode must not start while the first is running.
        let budget = CpuBudget::new(1);
        block_on(async {
            let first = budget.acquire().await.expect("first acquire should succeed");

            // Nothing is available, so a second acquire cannot complete. Proven by
            // it still being pending after a timeout rather than by a sleep race.
            let pending = budget.acquire();
            tokio::select! {
                _ = pending => panic!("second acquire must not succeed while the first is held"),
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
            }

            drop(first);

            // Now it completes. If the guard had not returned its permit this would
            // hang, so the test is bounded by a timeout.
            let second = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                budget.acquire(),
            )
            .await
            .expect("acquire must succeed once the permit is released")
            .expect("acquire must not error on an open budget");
            drop(second);
        });
    }

    #[test]
    fn two_permits_can_be_held_at_once_when_the_limit_allows_it() {
        let budget = CpuBudget::new(2);
        block_on(async {
            let a = budget.acquire().await.expect("first");
            let b = budget.acquire().await.expect("second");
            assert_eq!(budget.available(), 0);
            drop((a, b));
        });
        assert_eq!(budget.available(), 2);
    }

    #[test]
    fn a_zero_limit_is_raised_to_one_rather_than_blocking_forever() {
        // A budget of zero would never grant a permit, so no video would ever be
        // encoded and nothing would say why.
        let budget = CpuBudget::new(0);
        block_on(async {
            let permit = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                budget.acquire(),
            )
            .await
            .expect("a zero limit must not deadlock")
            .expect("should grant a permit");
            drop(permit);
        });
    }

    #[test]
    fn acquiring_from_a_closed_budget_returns_an_error() {
        // The failure the contract requires to be surfaced: no permit is coming,
        // and the caller must not fall through into encoding anyway.
        let budget = CpuBudget::new(1);
        budget.close();
        block_on(async {
            let result = budget.acquire().await;
            assert!(
                result.is_err(),
                "a closed budget must report an error, not grant a permit"
            );
        });
    }

    // The shared handle. Its whole job is to build the pool exactly once, from a
    // value that is only readable after the frontend has loaded the database.

    #[test]
    fn the_shared_budget_grants_a_permit_on_first_use() {
        let shared = SharedCpuBudget::default();
        block_on(async {
            let permit = shared.acquire(1).await.expect("first acquire should succeed");
            drop(permit);
        });
    }

    #[test]
    fn the_first_limit_wins_and_later_ones_are_ignored() {
        // Guards the documented contract. If a later call could widen the pool, the
        // settings page's promise that the change needs a restart would be false,
        // and the budget could be enlarged mid-run by an unrelated caller.
        let shared = SharedCpuBudget::default();
        block_on(async {
            let held = shared.acquire(1).await.expect("builds the pool with 1 permit");

            // A later call asking for 8 must still find only the one permit, which
            // is currently held — so it stays pending.
            tokio::select! {
                _ = shared.acquire(8) => {
                    panic!("a later limit must not enlarge the pool")
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
            }

            drop(held);

            let after = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                shared.acquire(8),
            )
            .await
            .expect("must succeed once the first permit is released")
            .expect("open budget");
            drop(after);
        });
    }

    #[test]
    fn a_closed_budget_reports_why_it_refused() {
        // The message reaches the user as the command's error string, so it has to
        // say what happened rather than being empty.
        let budget = CpuBudget::new(1);
        budget.close();
        block_on(async {
            let message = budget.acquire().await.expect_err("should be an error");
            assert!(
                message.contains("CPU budget"),
                "unhelpful error message: {:?}",
                message
            );
        });
    }
}
