use anyhow::anyhow;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub enum ChallengeSolution<T> {
    Solved(T),
    Cancel,
}

#[derive(Clone)]
pub struct AsyncChallengeState<T> {
    mutex: Arc<Mutex<Option<PendingChallenge<T>>>>,
}

struct PendingChallenge<T> {
    sender: oneshot::Sender<ChallengeSolution<T>>,
}

impl<T: Send + 'static> AsyncChallengeState<T> {
    pub fn new() -> Self {
        Self {
            mutex: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn request_solve<F, Fut>(
        &self,
        payload: String,
        when_ready: F,
    ) -> anyhow::Result<ChallengeSolution<T>>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        self.cancel().await;
        let (tx, rx) = oneshot::channel();

        {
            let mut lock = self.mutex.lock().unwrap();
            *lock = Some(PendingChallenge { sender: tx });
        }

        when_ready(payload).await;

        match rx.await {
            Ok(solution) => Ok(solution),
            Err(e) => Err(anyhow!(
                "Async challenge channel closed unexpectedly or sender dropped: {}",
                e
            )),
        }
    }

    pub async fn solve(&self, solution: T) {
        let mut lock = self.mutex.lock().unwrap();
        if let Some(pending) = lock.take() {
            let _ = pending.sender.send(ChallengeSolution::Solved(solution));
        }
    }

    pub async fn cancel(&self) {
        let mut lock = self.mutex.lock().unwrap();
        if let Some(pending) = lock.take() {
            let _ = pending.sender.send(ChallengeSolution::Cancel);
        }
    }
}

impl<T: Send + 'static> Default for AsyncChallengeState<T> {
    fn default() -> Self {
        Self::new()
    }
}
