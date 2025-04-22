use std::{fs, panic, sync::Arc};

use containerd_client::{
    Client,
    services::v1::{
        Container, CreateContainerRequest, CreateTaskRequest, DeleteContainerRequest,
        DeleteTaskRequest, KillRequest, ListContainersRequest, ListNamespacesRequest,
        ListTasksRequest, ListTasksResponse, StartRequest, WaitRequest, WaitResponse,
        container::Runtime,
        snapshots::{MountsRequest, PrepareSnapshotRequest},
    },
    tonic::Request,
    types::{Mount, v1::Process},
    with_namespace,
};
use prost_types::Any;
use sha2::{Digest, Sha256};
use tokio::{
    sync::OnceCell,
    time::{Duration, timeout},
};

use crate::{GLOBAL_NETNS_MAP, NetworkConfig, image_manager::ImageManager, spec::generate_spec};

pub(super) static CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

#[derive(Debug)]
pub struct ContainerdManager;

impl ContainerdManager {
    pub async fn init(socket_path: &str) {
        if let Err(e) = CLIENT.set(Arc::new(Client::from_path(socket_path).await.unwrap())) {
            panic!("Failed to set client: {}", e);
        }
        let _ = cni::init_net_work();
        log::info!("ContainerdManager initialized");
    }

    async fn get_client() -> Arc<Client> {
        CLIENT
            .get()
            .unwrap_or_else(|| panic!("Client not initialized, Please run init first"))
            .clone()
    }

    /// 创建容器
    pub async fn create_container(
        image_name: &str,
        cid: &str,
        ns: &str,
    ) -> Result<(), ContainerdError> {
        Self::prepare_snapshot(image_name, cid, ns)
            .await
            .map_err(|e| {
                log::error!("Failed to create container: {}", e);
                ContainerdError::CreateContainerError(e.to_string())
            })?;

        let spec = Self::get_spec(cid, ns, image_name).unwrap();

        let container = Container {
            id: cid.to_string(),
            image: image_name.to_string(),
            runtime: Some(Runtime {
                name: "io.containerd.runc.v2".to_string(),
                options: None,
            }),
            spec,
            snapshotter: "overlayfs".to_string(),
            snapshot_key: cid.to_string(),
            ..Default::default()
        };

        Self::do_create_container(container, ns).await?;

        Ok(())
    }

    async fn do_create_container(container: Container, ns: &str) -> Result<(), ContainerdError> {
        let mut cc = Self::get_client().await.containers();
        let req = CreateContainerRequest {
            container: Some(container),
        };
        let _resp = cc.create(with_namespace!(req, ns)).await.map_err(|e| {
            log::error!("Failed to create container: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;
        Ok(())
    }

    /// 删除容器
    pub async fn delete_container(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let container_list = Self::list_container(ns).await?;
        if !container_list.iter().any(|container| container.id == cid) {
            log::info!("Container {} not found", cid);
            return Ok(());
        }

        let resp = Self::list_task_by_cid(cid, ns).await?;
        if let Some(task) = resp.tasks.iter().find(|task| task.id == cid) {
            log::info!("Task found: {}, Status: {}", task.id, task.status);
            // TASK_UNKNOWN (0) — 未知状态
            // TASK_CREATED (1) — 任务已创建
            // TASK_RUNNING (2) — 任务正在运行
            // TASK_STOPPED (3) — 任务已停止
            // TASK_EXITED (4) — 任务已退出
            // TASK_PAUSED (5) — 任务已暂停
            // TASK_FAILED (6) — 任务失败
            Self::kill_task_with_timeout(cid, ns).await?;
        }

        Self::do_delete_container(cid, ns).await?;

        Self::remove_cni_network(cid, ns).map_err(|e| {
            log::error!("Failed to remove CNI network: {}", e);
            ContainerdError::CreateTaskError(e.to_string())
        })?;
        Ok(())
    }

    async fn do_delete_container(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut cc = Self::get_client().await.containers();
        let delete_request = DeleteContainerRequest {
            id: cid.to_string(),
        };

        let _ = cc
            .delete(with_namespace!(delete_request, ns))
            .await
            .expect("Failed to delete container");

        Ok(())
    }

    /// 创建并启动任务
    pub async fn new_task(cid: &str, ns: &str, image_name: &str) -> Result<(), ContainerdError> {
        let mounts = Self::get_mounts(cid, ns).await?;
        Self::prepare_cni_network(cid, ns, image_name)?;
        Self::do_create_task(cid, ns, mounts).await?;
        Self::do_start_task(cid, ns).await?;
        Ok(())
    }

    async fn do_start_task(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = Self::get_client().await.tasks();
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
        cid: &str,
        ns: &str,
        rootfs: Vec<Mount>,
    ) -> Result<(), ContainerdError> {
        let mut tc = Self::get_client().await.tasks();
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

    pub async fn get_task(cid: &str, ns: &str) -> Result<Process, ContainerdError> {
        let mut tc = Self::get_client().await.tasks();

        let request = ListTasksRequest {
            filter: format!("container=={}", cid),
        };

        let response = tc.list(with_namespace!(request, ns)).await.map_err(|e| {
            log::error!("Failed to list tasks: {}", e);
            ContainerdError::GetContainerListError(e.to_string())
        })?;
        let tasks = response.into_inner().tasks;

        let task =
            tasks
                .into_iter()
                .find(|task| task.id == cid)
                .ok_or_else(|| -> ContainerdError {
                    log::error!("Task not found for container: {}", cid);
                    ContainerdError::CreateTaskError("Task not found".to_string())
                })?;

        Ok(task)
    }

    async fn get_mounts(cid: &str, ns: &str) -> Result<Vec<Mount>, ContainerdError> {
        let mut sc = Self::get_client().await.snapshots();
        let req = MountsRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
        };
        let mounts = sc
            .mounts(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to get mounts: {}", e);
                ContainerdError::CreateTaskError(e.to_string())
            })?
            .into_inner()
            .mounts;

        Ok(mounts)
    }

    async fn list_task_by_cid(cid: &str, ns: &str) -> Result<ListTasksResponse, ContainerdError> {
        let mut c = Self::get_client().await.tasks();

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

    async fn do_kill_task(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = Self::get_client().await.tasks();
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

    async fn do_kill_task_force(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = Self::get_client().await.tasks();
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

    async fn do_delete_task(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let mut c = Self::get_client().await.tasks();
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

    async fn do_wait_task(cid: &str, ns: &str) -> Result<WaitResponse, ContainerdError> {
        let mut c = Self::get_client().await.tasks();
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
    pub async fn kill_task_with_timeout(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        let kill_timeout = Duration::from_secs(5);

        let wait_future = Self::do_wait_task(cid, ns);

        Self::do_kill_task(cid, ns).await?;

        match timeout(kill_timeout, wait_future).await {
            Ok(Ok(_)) => {
                // 正常退出，尝试删除任务
                Self::do_delete_task(cid, ns).await?;
            }
            Ok(Err(e)) => {
                // wait 报错
                log::error!("Error while waiting for task {}: {}", cid, e);
            }
            Err(_) => {
                // 超时，强制 kill
                log::warn!("Task {} did not exit in time, sending SIGKILL", cid);
                Self::do_kill_task_force(cid, ns).await?;

                // 尝试删除任务
                if let Err(e) = Self::do_delete_task(cid, ns).await {
                    log::error!("Failed to delete task {} after SIGKILL: {}", cid, e);
                }
            }
        }

        Ok(())
    }

    /// 获取一个容器
    pub async fn load_container(cid: &str, ns: &str) -> Result<Option<Container>, ContainerdError> {
        let container_list = Self::list_container(ns).await?;
        let container = container_list
            .into_iter()
            .find(|container| container.id == cid);

        Ok(container)
    }

    /// 获取容器列表
    pub async fn list_container(ns: &str) -> Result<Vec<Container>, ContainerdError> {
        let mut cc = Self::get_client().await.containers();

        let request = ListContainersRequest {
            ..Default::default()
        };

        let resp = cc.list(with_namespace!(request, ns)).await.map_err(|e| {
            log::error!("Failed to list containers: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;

        Ok(resp.into_inner().containers)
    }

    pub async fn list_container_into_string(ns: &str) -> Result<Vec<String>, ContainerdError> {
        let mut cc = Self::get_client().await.containers();

        let request = ListContainersRequest {
            ..Default::default()
        };

        let resp = cc.list(with_namespace!(request, ns)).await.map_err(|e| {
            log::error!("Failed to list containers: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;

        Ok(resp
            .into_inner()
            .containers
            .into_iter()
            .map(|container| container.id)
            .collect())
    }

    async fn prepare_snapshot(
        image_name: &str,
        cid: &str,
        ns: &str,
    ) -> Result<(), ContainerdError> {
        let parent_snapshot = Self::get_parent_snapshot(image_name).await?;
        Self::do_prepare_snapshot(cid, ns, parent_snapshot).await?;

        Ok(())
    }

    async fn do_prepare_snapshot(
        cid: &str,
        ns: &str,
        parent_snapshot: String,
    ) -> Result<(), ContainerdError> {
        let req = PrepareSnapshotRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
            parent: parent_snapshot,
            ..Default::default()
        };
        let client = Self::get_client().await;
        let _resp = client
            .snapshots()
            .prepare(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to prepare snapshot: {}", e);
                ContainerdError::CreateSnapshotError(e.to_string())
            })?;

        Ok(())
    }

    async fn get_parent_snapshot(image_name: &str) -> Result<String, ContainerdError> {
        let config = ImageManager::get_image_config(image_name).map_err(|e| {
            log::error!("Failed to get image config: {}", e);
            ContainerdError::GetParentSnapshotError(e.to_string())
        })?;

        if config.rootfs().diff_ids().is_empty() {
            log::error!("Image config has no diff_ids for image: {}", image_name);
            return Err(ContainerdError::GetParentSnapshotError(
                "No diff_ids found in image config".to_string(),
            ));
        }

        let mut iter = config.rootfs().diff_ids().iter();
        let mut ret = iter
            .next()
            .map_or_else(String::new, |layer_digest| layer_digest.clone());

        for layer_digest in iter {
            let mut hasher = Sha256::new();
            hasher.update(ret.as_bytes());
            ret.push_str(&format!(",{}", layer_digest));
            hasher.update(" ");
            hasher.update(layer_digest);
            let digest = ::hex::encode(hasher.finalize());
            ret = format!("sha256:{digest}");
        }
        Ok(ret)
    }

    fn get_spec(cid: &str, ns: &str, image_name: &str) -> Result<Option<Any>, ContainerdError> {
        let config = ImageManager::get_runtime_config(image_name).unwrap();
        let spec_path = generate_spec(cid, ns, &config).map_err(|e| {
            log::error!("Failed to generate spec: {}", e);
            ContainerdError::GenerateSpecError(e.to_string())
        })?;
        let spec = fs::read_to_string(spec_path).unwrap();
        let spec = Any {
            type_url: "types.containerd.io/opencontainers/runtime-spec/1/Spec".to_string(),
            value: spec.into_bytes(),
        };
        Ok(Some(spec))
    }

    /// 为一个容器准备cni网络并写入全局map中
    fn prepare_cni_network(cid: &str, ns: &str, image_name: &str) -> Result<(), ContainerdError> {
        let ip = cni::create_cni_network(cid.to_string(), ns.to_string()).map_err(|e| {
            log::error!("Failed to create CNI network: {}", e);
            ContainerdError::CreateTaskError(e.to_string())
        })?;
        let ports = ImageManager::get_runtime_config(image_name).unwrap().ports;
        let network_config = NetworkConfig::new(ip, ports);
        Self::save_container_network_config(cid, network_config);
        Ok(())
    }

    /// 删除cni网络，删除全局map中的网络配置
    fn remove_cni_network(cid: &str, ns: &str) -> Result<(), ContainerdError> {
        cni::delete_cni_network(ns, cid);
        Self::remove_container_network_config(cid);
        Ok(())
    }

    fn save_container_network_config(cid: &str, net_conf: NetworkConfig) {
        let mut map = GLOBAL_NETNS_MAP.write().unwrap();
        map.insert(cid.to_string(), net_conf);
    }

    pub fn get_address(cid: &str) -> String {
        let map = GLOBAL_NETNS_MAP.read().unwrap();
        let addr = map.get(cid).map(|net_conf| net_conf.get_address());
        addr.unwrap_or_default()
    }

    fn remove_container_network_config(cid: &str) {
        let mut map = GLOBAL_NETNS_MAP.write().unwrap();
        map.remove(cid);
    }

    pub async fn list_namespaces() -> Result<Vec<String>, ContainerdError> {
        let mut c = Self::get_client().await.namespaces();
        let req = ListNamespacesRequest {
            ..Default::default()
        };
        let resp = c.list(req).await.map_err(|e| {
            log::error!("Failed to list namespaces: {}", e);
            ContainerdError::GetContainerListError(e.to_string())
        })?;
        Ok(resp
            .into_inner()
            .namespaces
            .into_iter()
            .map(|ns| ns.name)
            .collect())
    }

    pub async fn pause_task() {
        todo!()
    }

    pub async fn resume_task() {
        todo!()
    }
}

#[derive(Debug)]
pub enum ContainerdError {
    CreateContainerError(String),
    CreateSnapshotError(String),
    GetParentSnapshotError(String),
    GenerateSpecError(String),
    DeleteContainerError(String),
    GetContainerListError(String),
    KillTaskError(String),
    DeleteTaskError(String),
    WaitTaskError(String),
    CreateTaskError(String),
    StartTaskError(String),
    #[allow(dead_code)]
    OtherError,
}

impl std::fmt::Display for ContainerdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerdError::CreateContainerError(msg) => {
                write!(f, "Failed to create container: {}", msg)
            }
            ContainerdError::CreateSnapshotError(msg) => {
                write!(f, "Failed to create snapshot: {}", msg)
            }
            ContainerdError::GetParentSnapshotError(msg) => {
                write!(f, "Failed to get parent snapshot: {}", msg)
            }
            ContainerdError::GenerateSpecError(msg) => {
                write!(f, "Failed to generate spec: {}", msg)
            }
            ContainerdError::DeleteContainerError(msg) => {
                write!(f, "Failed to delete container: {}", msg)
            }
            ContainerdError::GetContainerListError(msg) => {
                write!(f, "Failed to get container list: {}", msg)
            }
            ContainerdError::KillTaskError(msg) => {
                write!(f, "Failed to kill task: {}", msg)
            }
            ContainerdError::DeleteTaskError(msg) => {
                write!(f, "Failed to delete task: {}", msg)
            }
            ContainerdError::WaitTaskError(msg) => {
                write!(f, "Failed to wait task: {}", msg)
            }
            ContainerdError::CreateTaskError(msg) => {
                write!(f, "Failed to create task: {}", msg)
            }
            ContainerdError::StartTaskError(msg) => {
                write!(f, "Failed to start task: {}", msg)
            }
            ContainerdError::OtherError => write!(f, "Other error happened"),
        }
    }
}

impl std::error::Error for ContainerdError {}
