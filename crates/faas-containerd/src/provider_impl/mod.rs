pub mod function;

use std::collections::HashMap;

use provider::{handlers::function::{DeleteError, DeployError, ResolveError}, types::function::{Deployment, Query}};
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::task::TaskTracker;
use std::sync::Mutex;

use crate::impls::function::FunctionInstance;

pub struct CtrdProvider {
    pub ctr_instance_map: Mutex<HashMap<Query, FunctionInstance>>,
    // pub backend: ContainerdService,
    // pub self_ref: Weak<Self>,
    pub task_tracker: tokio_util::task::TaskTracker,
}

impl Drop for CtrdProvider {
    fn drop(&mut self) {
        let tracker = TaskTracker::new();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        self.ctr_instance_map
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_q, container)| {
                tracker.spawn(async move {
                    let _ = container.delete().await;
                });
            });
        let signal_handle = tokio::spawn(async move {
            // listen for shutdown signals
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigquit = signal(SignalKind::quit()).unwrap();
            tokio::select! {
                _ = sigint.recv() => println!("SIGINT received, starting graceful shutdown..."),
                _ = sigterm.recv() => println!("SIGTERM received, starting graceful shutdown..."),
                _ = sigquit.recv() => println!("SIGQUIT received, starting graceful shutdown..."),
            }
            tracker.close();
            tracker.wait().await;
            log::info!("All tasks have been closed.");
        });
        let _ = rt.block_on(signal_handle);
    }
}

impl provider::provider::Provider for CtrdProvider {
    fn resolve(
        &self,
        function: Query,
    ) -> impl std::future::Future<
        Output = Result<url::Url, ResolveError>,
    > + Send {
        self._resolve(function)
    }

    fn deploy(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), DeployError>> + Send
    {
        self._deploy(param)
    }

    fn delete(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<(), DeleteError>> + Send
    {
        self._delete(function)
    }
}
