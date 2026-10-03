//! Démarrage in-process du runtime Apparatus (noms hors allowlist T7).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apparatus_contracts::ApparatusRuntime;
use manifesto_infra::apparatus_runtime::run_tick_loop;
use sea_orm::DatabaseConnection;
use tokio::task::JoinHandle;

/// Handle de la tâche de fond Apparatus.
pub struct ApparatusRuntimeHandle {
    join: JoinHandle<()>,
    live: Arc<AtomicBool>,
}

impl ApparatusRuntimeHandle {
    /// `true` tant que la tâche n'est pas arrêtée.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst) && !self.join.is_finished()
    }

    /// Arrête la tâche.
    pub fn abort(&self) {
        self.live.store(false, Ordering::SeqCst);
        self.join.abort();
    }

    /// Flag partagé avec `/ready` (`false` après [`Self::abort`]).
    #[must_use]
    pub fn live_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.live)
    }
}

/// Démarre la boucle d'intervalle in-process.
pub fn start_apparatus_runtime(
    db: DatabaseConnection,
    runtime: Arc<dyn ApparatusRuntime>,
    owner: String,
    tick_interval: Duration,
) -> ApparatusRuntimeHandle {
    let live = Arc::new(AtomicBool::new(true));
    let live_flag = live.clone();
    let join = tokio::spawn(async move {
        let _clear_on_exit = ClearLiveOnDrop(live_flag);
        run_tick_loop(db, runtime, owner, tick_interval).await;
    });
    ApparatusRuntimeHandle { join, live }
}

/// Met le flag à `false` si la tâche finit ou panique (aligné sur `is_live()`).
struct ClearLiveOnDrop(Arc<AtomicBool>);

impl Drop for ClearLiveOnDrop {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
