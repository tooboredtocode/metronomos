use std::time::Duration;

use metronomos_pulse::dependency::FnDependency;
use tokio::sync::mpsc;

use crate::lifecycle::Lifecycle;
use crate::lifecycle::context::LifecycleContextManager;
use crate::lifecycle::hook::LifecycleHook;

const DEFAULT_LIFECYCLE_TIMEOUT: Duration = Duration::ZERO;
const MAX_CHANNEL_SIZE: usize = u16::MAX as usize;

pub(crate) struct LifecycleInner {
    sink: mpsc::Sender<LifecycleHook>,
    source: mpsc::Receiver<LifecycleHook>,
    default_timeout: Duration,
    registered_hooks: Vec<LifecycleHook>,
}

impl LifecycleInner {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel(MAX_CHANNEL_SIZE);
        Self {
            sink: tx,
            source: rx,
            default_timeout: DEFAULT_LIFECYCLE_TIMEOUT,
            registered_hooks: Vec::new(),
        }
    }

    pub(crate) fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    pub(crate) fn as_provide(&self) -> impl FnDependency<(), Value = Lifecycle> {
        let lifecycle = Lifecycle {
            sink: self.sink.clone(),
        };
        move || Ok(lifecycle.clone())
    }

    fn update_hooks(&mut self) {
        while let Ok(hook) = self.source.try_recv() {
            self.registered_hooks.push(hook);
        }
    }

    pub(crate) fn start_hooks(&mut self) -> LifecycleContextManager {
        self.update_hooks();

        let mut ctx = LifecycleContextManager::new();

        for hook in &self.registered_hooks {
            hook.start_on(&mut ctx, self.default_timeout);
        }

        ctx
    }
}
