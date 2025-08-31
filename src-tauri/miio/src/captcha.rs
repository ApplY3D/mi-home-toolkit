use anyhow::anyhow;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub enum CaptchaSolution {
    Solved(String),
    Cancel,
}

#[derive(Clone)]
pub struct CaptchaState {
    mutex: Arc<Mutex<Option<CaptchaPending>>>,
}

struct CaptchaPending {
    sender: oneshot::Sender<CaptchaSolution>,
}

impl CaptchaState {
    pub fn new() -> Self {
        Self {
            mutex: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn captcha_request_solve<F, Fut>(
        &self,
        captcha_url: String,
        when_ready: F,
    ) -> anyhow::Result<CaptchaSolution>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        self.cancel().await;
        let (tx, rx) = oneshot::channel();

        {
            let mut lock = self.mutex.lock().unwrap();
            *lock = Some(CaptchaPending { sender: tx });
        }

        when_ready(captcha_url).await;

        match rx.await {
            Ok(solution) => Ok(solution),
            Err(e) => Err(anyhow!(
                "Captcha solution channel closed unexpectedly or sender dropped: {}",
                e
            )),
        }
    }

    pub async fn solve(&self, solution: String) {
        let mut lock = self.mutex.lock().unwrap();
        if let Some(pending) = lock.take() {
            let _ = pending
                .sender
                .send(CaptchaSolution::Solved(solution.to_string()));
        }
    }

    pub async fn cancel(&self) {
        let mut lock = self.mutex.lock().unwrap();
        if let Some(pending) = lock.take() {
            let _ = pending.sender.send(CaptchaSolution::Cancel);
        }
    }
}

impl Default for CaptchaState {
    fn default() -> Self {
        Self::new()
    }
}
