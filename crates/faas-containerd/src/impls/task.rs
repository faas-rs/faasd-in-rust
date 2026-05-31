use containerd_client::{
    services::v1::{
        CreateTaskRequest, DeleteTaskRequest, GetRequest, KillRequest, ListTasksRequest,
        WaitRequest, WaitResponse,
    },
    types::{Mount, v1::Process},
    with_namespace,
};
use derive_more::Display;
use gateway::types::{DeleteError, DeployError};
use tonic::Request;

use super::{ContainerdService, cni::Endpoint};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Display)]
pub enum TaskError {
    NotFound,
    AlreadyExists,
    InvalidArgument,
    Internal(String),
}

impl From<tonic::Status> for TaskError {
    fn from(status: tonic::Status) -> Self {
        use tonic::Code::*;
        match status.code() {
            NotFound => TaskError::NotFound,
            AlreadyExists => TaskError::AlreadyExists,
            InvalidArgument => TaskError::InvalidArgument,
            _ => TaskError::Internal(status.message().to_string()),
        }
    }
}

impl From<TaskError> for DeployError {
    fn from(e: TaskError) -> DeployError {
        DeployError::Internal(e.to_string())
    }
}

impl From<TaskError> for DeleteError {
    fn from(e: TaskError) -> DeleteError {
        match e {
            TaskError::NotFound => DeleteError::NotFound(e.to_string()),
            e => DeleteError::Internal(e.to_string()),
        }
    }
}

impl ContainerdService {
    /// 创建并启动任务
    pub async fn new_task(&self, mounts: Vec<Mount>, endpoint: &Endpoint) -> Result<(), TaskError> {
        let cid = endpoint.to_string();
        let ns = &endpoint.namespace;
        self.do_create_task(&cid, ns, mounts).await?;
        self.do_start_task(&cid, ns).await?;
        Ok(())
    }

    async fn do_start_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut c = self.client.tasks();
        let start_request = containerd_client::services::v1::StartRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        c.start(with_namespace!(start_request, ns)).await?;
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
        let mut tc = self.client.tasks();
        let req = GetRequest {
            container_id: endpoint.to_string(),
            ..Default::default()
        };
        let resp = tc.get(with_namespace!(req, endpoint.namespace)).await?;
        let task = resp.into_inner().process.ok_or(TaskError::NotFound)?;
        Ok(task)
    }

    /// Check whether a task exists in containerd.
    pub async fn task_exists(&self, endpoint: &Endpoint) -> bool {
        self.get_task(endpoint).await.is_ok()
    }

    #[allow(dead_code)]
    async fn list_task_by_cid(
        &self,
        cid: &str,
        ns: &str,
    ) -> Result<containerd_client::services::v1::ListTasksResponse, TaskError> {
        let mut tc = self.client.tasks();
        let req = ListTasksRequest {
            filter: format!("id=={}", cid),
        };
        let resp = tc.list(with_namespace!(req, ns)).await?;
        Ok(resp.into_inner())
    }

    async fn do_kill_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut tc = self.client.tasks();
        let req = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            ..Default::default()
        };
        tc.kill(with_namespace!(req, ns)).await?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn do_kill_task_force(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut tc = self.client.tasks();
        let req = KillRequest {
            container_id: cid.to_string(),
            signal: 9,
            ..Default::default()
        };
        tc.kill(with_namespace!(req, ns)).await?;
        Ok(())
    }

    async fn do_delete_task(&self, cid: &str, ns: &str) -> Result<(), TaskError> {
        let mut tc = self.client.tasks();
        let req = DeleteTaskRequest {
            container_id: cid.to_string(),
        };
        tc.delete(with_namespace!(req, ns)).await?;
        Ok(())
    }

    async fn do_wait_task(&self, cid: &str, ns: &str) -> Result<WaitResponse, TaskError> {
        let mut tc = self.client.tasks();
        let req = WaitRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let resp = tc.wait(with_namespace!(req, ns)).await?;
        Ok(resp.into_inner())
    }

    /// 杀死并删除任务
    pub async fn kill_task_with_timeout(&self, endpoint: &Endpoint) -> Result<(), TaskError> {
        let cid = endpoint.to_string();
        let ns = &endpoint.namespace;
        let wait_future = self.do_wait_task(&cid, ns);
        self.do_kill_task(&cid, ns).await?;
        match wait_future.await {
            Ok(_) => {
                self.do_delete_task(&cid, ns).await?;
            }
            Err(e) => {
                log::error!("Error while waiting for task {}: {:?}", cid, e);
                return Err(e);
            }
        }
        Ok(())
    }
}
