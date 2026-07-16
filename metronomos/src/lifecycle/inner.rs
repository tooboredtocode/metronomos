use metronomos_pulse::dependency::FnDependency;
use tokio::sync::mpsc;

use crate::lifecycle::Lifecycle;
use crate::lifecycle::context::LifecycleContextManager;
use crate::lifecycle::hook::LifeCycleHook;

pub(crate) struct LifecycleInner {
    sink: mpsc::Sender<LifeCycleHook>,
    source: mpsc::Receiver<LifeCycleHook>,
    registered_hooks: Vec<LifeCycleHook>,
}

const MAX_CHANNEL_SIZE: usize = u16::MAX as usize;

impl LifecycleInner {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel(MAX_CHANNEL_SIZE);
        Self {
            sink: tx,
            source: rx,
            registered_hooks: Vec::new(),
        }
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
            hook.start(&mut ctx);
        }

        ctx
    }
}
