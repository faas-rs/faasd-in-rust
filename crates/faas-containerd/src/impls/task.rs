use std::time::Duration;

use containerd_client::{
    services::v1::{
        CreateTaskRequest, DeleteTaskRequest, KillRequest, ListTasksRequest, ListTasksResponse,
        StartRequest, WaitRequest, WaitResponse,
    },
    types::{Mount, v1::Process},
    with_namespace,
};
use tonic::Request;

use crate::impls::error::ContainerdError;

use super::{ContainerdService, cni::Endpoint};

impl ContainerdService {
    /// 创建并启动任务
    pub async fn new_task(
        &self,
        mounts: Vec<Mount>,
        endpoint: &Endpoint,
    ) -> Result<(), ContainerdError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        // let mounts = self.get_mounts(cid, ns).await?;
        self.do_create_task(cid, ns, mounts).await?;
        self.do_start_task(cid, ns).await?;
        Ok(())
    }

    async fn do_start_task(&self, cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c: containerd_client::services::v1::tasks_client::TasksClient<
            tonic::transport::Channel,
        > = self.client.tasks();
        let req = StartRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let _resp = c.start(with_namespace!(req, ns)).await.map_err(|e| {
            log::error!("Failed to start task: {}", e);
            ContainerdError::StartTaskError(e.to_string())
        })?;
        log::info!("Task: {:?} started", cid);

        Ok(())
    }

    async fn do_create_task(
        &self,
        cid: &str,
        ns: &str,
        rootfs: Vec<Mount>,
    ) -> Result<(), ContainerdError> {
        let mut tc = self.client.tasks();
        let create_request = CreateTaskRequest {
            container_id: cid.to_string(),
            rootfs,
            ..Default::default()
        };
        let _resp = tc
            .create(with_namespace!(create_request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to create task: {}", e);
                ContainerdError::CreateTaskError(e.to_string())
            })?;

        Ok(())
    }

    pub async fn get_task(&self, endpoint: &Endpoint) -> Result<Process, ContainerdError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        let mut tc = self.client.tasks();

        let request = ListTasksRequest {
            filter: format!("container=={}", cid),
        };

        let response = tc.list(with_namespace!(request, ns)).await.map_err(|e| {
            log::error!("Failed to list tasks: {}", e);
            ContainerdError::GetContainerListError(e.to_string())
        })?;
        let tasks = response.into_inner().tasks;

        let task = tasks
            .into_iter()
            .map(|task| {
                log::trace!("Task: {:?}", task);
                task
            })
            .find(|task| task.id == *cid)
            .ok_or_else(|| -> ContainerdError {
                log::error!("Task not found for container: {}", cid);
                ContainerdError::CreateTaskError("Task not found".to_string())
            })?;

        Ok(task)
    }

    #[allow(dead_code)]
    async fn list_task_by_cid(
        &self,
        cid: &str,
        ns: &str,
    ) -> Result<ListTasksResponse, ContainerdError> {
        let mut c = self.client.tasks();
        let request = ListTasksRequest {
            filter: format!("container=={}", cid),
        };
        let response = c
            .list(with_namespace!(request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to list tasks: {}", e);
                ContainerdError::GetContainerListError(e.to_string())
            })?
            .into_inner();
        Ok(response)
    }

    async fn do_kill_task(&self, cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = self.client.tasks();
        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to kill task: {}", e);
                ContainerdError::KillTaskError(e.to_string())
            })?;
        Ok(())
    }

    async fn do_kill_task_force(&self, cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = self.client.tasks();
        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 9,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to force kill task: {}", e);
                ContainerdError::KillTaskError(e.to_string())
            })?;
        Ok(())
    }

    async fn do_delete_task(&self, cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = self.client.tasks();
        let delete_request = DeleteTaskRequest {
            container_id: cid.to_string(),
        };
        c.delete(with_namespace!(delete_request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to delete task: {}", e);
                ContainerdError::DeleteTaskError(e.to_string())
            })?;
        Ok(())
    }

    async fn do_wait_task(&self, cid: &str, ns: &str) -> Result<WaitResponse, ContainerdError> {
        let mut c = self.client.tasks();
        let wait_request = WaitRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let resp = c
            .wait(with_namespace!(wait_request, ns))
            .await
            .map_err(|e| {
                log::error!("wait error: {}", e);
                ContainerdError::WaitTaskError(e.to_string())
            })?
            .into_inner();
        Ok(resp)
    }

    /// 杀死并删除任务
    pub async fn kill_task_with_timeout(&self, endpoint: &Endpoint) -> Result<(), ContainerdError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        let kill_timeout = Duration::from_secs(5);
        let wait_future = self.do_wait_task(cid, ns);
        self.do_kill_task(cid, ns).await?;
        match tokio::time::timeout(kill_timeout, wait_future).await {
            Ok(Ok(_)) => {
                // 正常退出，尝试删除任务
                self.do_delete_task(cid, ns).await?;
            }
            Ok(Err(e)) => {
                // wait 报错
                log::error!("Error while waiting for task {}: {:?}", cid, e);
            }
            Err(_) => {
                // 超时，强制 kill
                log::warn!("Task {} did not exit in time, sending SIGKILL", cid);
                self.do_kill_task_force(cid, ns).await?;
                // 尝试删除任务
                if let Err(e) = self.do_delete_task(cid, ns).await {
                    log::error!("Failed to delete task {} after SIGKILL: {:?}", cid, e);
                }
            }
        }
        Ok(())
    }
}
