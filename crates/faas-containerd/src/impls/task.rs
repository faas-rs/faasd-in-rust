use std::time::Duration;

use containerd_client::{
    services::v1::{
        CreateTaskRequest, DeleteTaskRequest, GetRequest, KillRequest, ListTasksRequest,
        ListTasksResponse, StartRequest, WaitRequest, WaitResponse,
    },
    types::{Mount, v1::Process},
    with_namespace,
};
use derive_more::Display;
use gateway::handlers::function::{DeleteError, DeployError};
use tonic::Request;

use super::{ContainerdService, cni::Endpoint};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Display)]
pub enum TaskError {
    NotFound,
    AlreadyExists,
    InvalidArgument,
    // PermissionDenied,
    Internal(String),
}

impl From<tonic::Status> for TaskError {
    fn from(status: tonic::Status) -> Self {
        use tonic::Code::*;
        match status.code() {
            NotFound => TaskError::NotFound,
            AlreadyExists => TaskError::AlreadyExists,
            InvalidArgument => TaskError::InvalidArgument,
            // PermissionDenied => TaskError::PermissionDenied,
            _ => TaskError::Internal(status.message().to_string()),
        }
    }
}

impl From<TaskError> for DeployError {
    fn from(e: TaskError) -> DeployError {
        match e {
            TaskError::InvalidArgument => DeployError::Invalid(e.to_string()),
            _ => DeployError::InternalError(e.to_string()),
        }
    }
}

impl From<TaskError> for DeleteError {
    fn from(e: TaskError) -> DeleteError {
        log::trace!("DeleteTaskError: {:?}", e);
        match e {
            TaskError::NotFound => DeleteError::NotFound(e.to_string()),
            TaskError::InvalidArgument => DeleteError::Invalid(e.to_string()),
            _ => DeleteError::Internal(e.to_string()),
        }
    }
}

impl ContainerdService {
    /// 创建并启动任务
    pub async fn new_task(&self, mounts: Vec<Mount>, endpoint: &Endpoint) -> Result<(), TaskError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        // let mounts = self.get_mounts(cid, ns).await?;
        self.do_create_task(cid, ns, mounts).await?;
        self.do_start_task(cid, ns).await?;
        Ok(())
    }

    async fn do_start_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut c: containerd_client::services::v1::tasks_client::TasksClient<
            tonic::transport::Channel,
        > = self.client.tasks();
        let req = StartRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let resp = c.start(with_namespace!(req, ns)).await?;
        log::debug!("Task: {:?} started", cid);
        log::trace!("Task start response: {:?}", resp);

        Ok(())
    }

    async fn do_create_task(
        &self,
        cid: &str,
        ns: &str,
        rootfs: Vec<Mount>,
    ) -> Result<(), TaskError> {
        let mut tc = self.client.tasks();
        let create_request = CreateTaskRequest {
            container_id: cid.to_string(),
            rootfs,
            ..Default::default()
        };
        let _resp = tc.create(with_namespace!(create_request, ns)).await?;

        Ok(())
    }

    pub async fn get_task(&self, endpoint: &Endpoint) -> Result<Process, TaskError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        let mut tc = self.client.tasks();

        let req = GetRequest {
            container_id: cid.clone(),
            ..Default::default()
        };

        let resp = tc.get(with_namespace!(req, ns)).await?;

        let task = resp.into_inner().process.ok_or(TaskError::NotFound)?;

        Ok(task)
    }

    #[allow(dead_code)]
    async fn list_task_by_cid(&self, cid: &str, ns: &str) -> Result<ListTasksResponse, TaskError> {
        let mut c = self.client.tasks();
        let request = ListTasksRequest {
            filter: format!("container=={}", cid),
        };
        let response = c.list(with_namespace!(request, ns)).await?.into_inner();
        Ok(response)
    }

    async fn do_kill_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut c = self.client.tasks();
        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, ns)).await?;
        Ok(())
    }

    async fn do_kill_task_force(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut c = self.client.tasks();
        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 9,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, ns)).await?;
        Ok(())
    }

    async fn do_delete_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut c = self.client.tasks();
        let delete_request = DeleteTaskRequest {
            container_id: cid.to_string(),
        };
        c.delete(with_namespace!(delete_request, ns)).await?;
        Ok(())
    }

    async fn do_wait_task(&self, cid: &str, ns: &str) -> Result<WaitResponse, TaskError> {
        let mut c = self.client.tasks();
        let wait_request = WaitRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let resp = c
            .wait(with_namespace!(wait_request, ns))
            .await?
            .into_inner();
        Ok(resp)
    }

    /// 杀死并删除任务
    pub async fn kill_task_with_timeout(&self, endpoint: &Endpoint) -> Result<(), TaskError> {
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
                return Err(e);
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
