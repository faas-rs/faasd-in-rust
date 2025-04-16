pub mod image_manager;
pub mod spec;
pub mod systemd;

use anyhow::anyhow;
use anyhow::{Context, Result};
use containerd_client::{
    Client,
    services::v1::{
        Container, CreateContainerRequest, CreateTaskRequest, DeleteContainerRequest,
        DeleteTaskRequest, KillRequest, ListContainersRequest, ListNamespacesRequest,
        ListTasksRequest, StartRequest, WaitRequest,
        container::Runtime,
        snapshots::{MountsRequest, PrepareSnapshotRequest},
    },
    tonic::Request,
    types::v1::Process,
    with_namespace,
};
use image_manager::ImageManager;
use prost_types::Any;
use sha2::{Digest, Sha256};
use spec::{DEFAULT_NAMESPACE, generate_spec};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time::timeout;
// config.json,dockerhub密钥
// const DOCKER_CONFIG_DIR: &str = "/var/lib/faasd/.docker/";

type NetnsMap = Arc<RwLock<HashMap<String, NetworkConfig>>>;
lazy_static::lazy_static! {
    static ref GLOBAL_NETNS_MAP: NetnsMap = Arc::new(RwLock::new(HashMap::new()));
}

type Err = Box<dyn std::error::Error>;

pub struct Service {
    pub client: Arc<Client>,
    netns_map: NetnsMap,
}

impl Service {
    pub async fn new(socket_path: &str) -> Result<Self> {
        let client = Client::from_path(socket_path).await.with_context(|| {
            format!("Failed to initialize Client, socket_path: {}", socket_path)
        })?;
        Ok(Service {
            client: Arc::new(client),
            netns_map: GLOBAL_NETNS_MAP.clone(), //全局网络空间映射
        })
    }
    pub async fn save_network_config(&self, cid: &str, net_conf: NetworkConfig) -> Result<()> {
        let mut map = self
            .netns_map
            .write()
            .map_err(|e| anyhow!("Failed to acquire write lock: {}", e))?;
        map.insert(cid.to_string(), net_conf);
        Ok(())
    }

    pub async fn get_network_config(&self, cid: &str) -> Result<Option<NetworkConfig>> {
        let map = self
            .netns_map
            .read()
            .map_err(|e| anyhow!("Failed to acquire read lock: {}", e))?;
        Ok(map.get(cid).cloned())
    }

    pub async fn get_ip(&self, cid: &str) -> Result<Option<String>> {
        let map = self
            .netns_map
            .read()
            .map_err(|e| anyhow!("Failed to acquire read lock: {}", e))?;
        Ok(map.get(cid).map(|net_conf| net_conf.get_ip()))
    }

    pub async fn get_address(&self, cid: &str) -> Result<Option<String>> {
        let map = self
            .netns_map
            .read()
            .map_err(|e| anyhow!("Failed to acquire read lock: {}", e))?;
        Ok(map.get(cid).map(|net_conf| net_conf.get_address()))
    }

    pub async fn remove_netns_ip(&self, cid: &str) -> Result<()> {
        let mut map = self
            .netns_map
            .write()
            .map_err(|e| anyhow!("Failed to acquire write lock: {}", e))?;
        map.remove(cid);
        Ok(())
    }

    async fn prepare_snapshot(
        &self,
        cid: &str,
        ns: &str,
        img_name: &str,
    ) -> Result<(), anyhow::Error> {
        let parent_snapshot = self
            .get_parent_snapshot(img_name)
            .await
            .with_context(|| format!("Failed to get parent snapshot for image {}", img_name))?;
        let req = PrepareSnapshotRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
            parent: parent_snapshot,
            ..Default::default()
        };
        let _resp = self
            .client
            .snapshots()
            .prepare(with_namespace!(req, ns))
            .await
            .with_context(|| {
                format!(
                    "Failed to prepare snapshot, cid: {}, namespace: {}",
                    cid, ns
                )
            })?;
        Ok(())
    }

    pub async fn create_container(&self, image_name: &str, cid: &str, ns: &str) -> Result<(), Err> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        self.prepare_snapshot(cid, ns, image_name)
            .await
            .with_context(|| {
                format!(
                    "Failed to prepare snapshot, cid: {}, ns: {}, image: {}",
                    cid, ns, image_name
                )
            })?;

        let config = ImageManager::get_runtime_config(image_name)
            .with_context(|| format!("Failed to get runtime config for image {}", image_name))?;

        let env = config.env;
        let args = config.args;

        let spec_path = generate_spec(cid, ns, args, env)
            .with_context(|| format!("Failed to generate spec file, cid: {}, ns: {}", cid, ns))?;
        let spec = fs::read_to_string(&spec_path)
            .with_context(|| format!("Failed to read spec file, path: {}", spec_path))?;

        let spec = Any {
            type_url: "types.containerd.io/opencontainers/runtime-spec/1/Spec".to_string(),
            value: spec.into_bytes(),
        };

        let mut containers_client = self.client.containers();
        let container = Container {
            id: cid.to_string(),
            image: image_name.to_string(),
            runtime: Some(Runtime {
                name: "io.containerd.runc.v2".to_string(),
                options: None,
            }),
            spec: Some(spec),
            snapshotter: "overlayfs".to_string(),
            snapshot_key: cid.to_string(),
            ..Default::default()
        };

        let req = CreateContainerRequest {
            container: Some(container),
        };

        containers_client
            .create(with_namespace!(req, namespace))
            .await
            .with_context(|| format!("Failed to create container, cid: {}, ns: {}", cid, ns))?;

        Ok(())
    }

    pub async fn remove_container(&self, cid: &str, ns: &str) -> anyhow::Result<()> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let request = ListContainersRequest {
            ..Default::default()
        };
        let mut cc = self.client.containers();

        let response = cc
            .list(with_namespace!(request, namespace))
            .await
            .with_context(|| format!("Failed to list containers, ns: {}", namespace))?
            .into_inner();
        let container = response
            .containers
            .iter()
            .find(|container| container.id == cid);

        if let Some(container) = container {
            let mut tc = self.client.tasks();

            let request = ListTasksRequest {
                filter: format!("container=={}", cid),
            };
            let response = tc
                .list(with_namespace!(request, namespace))
                .await
                .with_context(|| format!("Failed to list tasks, cid: {}, ns: {}", cid, namespace))?
                .into_inner();
            log::info!("Tasks: {:?}", response.tasks);
            drop(tc);

            if let Some(task) = response.tasks.iter().find(|task| task.id == container.id) {
                log::info!("Task found: {}, Status: {}", task.id, task.status);

                // TASK_UNKNOWN (0) — 未知状态
                // TASK_CREATED (1) — 任务已创建
                // TASK_RUNNING (2) — 任务正在运行
                // TASK_STOPPED (3) — 任务已停止
                // TASK_EXITED (4) — 任务已退出
                // TASK_PAUSED (5) — 任务已暂停
                // TASK_FAILED (6) — 任务失败
                if let Err(e) = self.kill_task(task.id.to_string(), ns).await {
                    log::warn!("kill_task failed: {}", e);
                }
                if let Err(e) = self.delete_task(&task.id, ns).await {
                    log::warn!("delete_task failed: {}", e);
                }
            }

            let delete_request = DeleteContainerRequest {
                id: container.id.clone(),
            };

            cc.delete(with_namespace!(delete_request, namespace))
                .await
                .with_context(|| {
                    format!(
                        "Failed to delete container, cid: {}, ns: {}",
                        cid, namespace
                    )
                })?;

            if let Err(e) = self.remove_netns_ip(cid).await {
                log::warn!("remove_netns_ip failed: {}", e);
            }

            log::info!("Container: {:?} deleted", cc);
        } else {
            return Err(anyhow::anyhow!("Container not found {}", cid));
        }
        drop(cc);
        Ok(())
    }

    pub async fn create_and_start_task(&self, cid: &str, ns: &str, img_name: &str) -> Result<()> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        self.create_task(cid, namespace, img_name)
            .await
            .with_context(|| {
                format!(
                    "Failed to create task, cid: {}, ns: {}, img: {}",
                    cid, ns, img_name
                )
            })?;

        self.start_task(cid, namespace)
            .await
            .with_context(|| format!("Failed to start task, cid: {}, ns: {}", cid, ns))?;

        Ok(())
    }
    async fn create_task(&self, cid: &str, ns: &str, img_name: &str) -> Result<u32> {
        let mut sc = self.client.snapshots();
        let req = MountsRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
        };

        let mounts = sc
            .mounts(with_namespace!(req, ns))
            .await
            .with_context(|| format!("Failed to get mounts, cid: {}, ns: {}", cid, ns))?
            .into_inner()
            .mounts;

        log::info!("mounts ok");
        drop(sc);
        log::info!("drop sc ok");

        cni::init_net_work().with_context(|| "Failed to initialize network")?;
        log::info!("init_net_work ok");

        let (ip, path) = cni::create_cni_network(cid.to_string(), ns.to_string())
            .with_context(|| format!("Failed to create CNI network, cid: {}, ns: {}", cid, ns))?;

        let ports = ImageManager::get_runtime_config(img_name)
            .with_context(|| format!("Failed to get runtime config for image {}", img_name))?
            .ports;

        let network_config = NetworkConfig::new(path, ip, ports);
        log::info!("create_cni_network ok");

        self.save_network_config(cid, network_config.clone())
            .await
            .with_context(|| format!("Failed to save network config, cid: {}", cid))?;
        log::info!("save_netns_ip ok, netconfig: {:?}", network_config);

        let mut tc = self.client.tasks();
        let req = CreateTaskRequest {
            container_id: cid.to_string(),
            rootfs: mounts,
            ..Default::default()
        };
        let resp = tc
            .create(with_namespace!(req, ns))
            .await
            .with_context(|| format!("Failed to create task, cid: {}, ns: {}", cid, ns))?;
        let pid = resp.into_inner().pid;
        Ok(pid)
    }

    async fn start_task(&self, cid: &str, ns: &str) -> Result<()> {
        let req = StartRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        self.client
            .tasks()
            .start(with_namespace!(req, ns))
            .await
            .with_context(|| format!("Failed to start task, cid: {}, ns: {}", cid, ns))?;
        log::info!("Task: {:?} started", cid);

        Ok(())
    }

    pub async fn kill_task(&self, cid: String, ns: &str) -> Result<()> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let mut c = self.client.tasks();
        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, namespace))
            .await
            .with_context(|| format!("kill_task failed, cid: {}, ns: {}", cid, namespace))?;

        Ok(())
    }
    pub async fn pause_task() {
        todo!()
    }
    pub async fn resume_task() {
        todo!()
    }
    pub async fn delete_task(&self, cid: &str, ns: &str) -> Result<()> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let mut c = self.client.tasks();
        let time_out = Duration::from_secs(30);
        let wait_result = timeout(time_out, async {
            let wait_request = WaitRequest {
                container_id: cid.to_string(),
                ..Default::default()
            };

            c.wait(with_namespace!(wait_request, namespace))
                .await
                .with_context(|| {
                    format!(
                        "Failed to wait for task to finish, cid: {}, ns: {}",
                        cid, namespace
                    )
                })?;
            Ok::<(), anyhow::Error>(())
        })
        .await;

        log::info!("after wait");

        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, namespace))
            .await
            .with_context(|| format!("kill_task failed, cid: {}, ns: {}", cid, namespace))?;

        match wait_result {
            Ok(Ok(_)) => {
                let req = DeleteTaskRequest {
                    container_id: cid.to_string(),
                };
                c.delete(with_namespace!(req, namespace))
                    .await
                    .with_context(|| {
                        format!("Failed to delete task, cid: {}, ns: {}", cid, namespace)
                    })?;
                log::info!("Task: {:?} deleted", cid);
                Ok(())
            }
            Ok(Err(e)) => {
                log::error!("Wait task failed: {}", e);
                Err(e)
            }
            Err(_) => {
                let kill_request = KillRequest {
                    container_id: cid.to_string(),
                    signal: 9,
                    all: true,
                    ..Default::default()
                };
                c.kill(with_namespace!(kill_request, namespace))
                    .await
                    .with_context(|| {
                        format!("Force kill task failed, cid: {}, ns: {}", cid, namespace)
                    })?;
                log::info!("Task: {:?} force killed", cid);
                Ok(())
            }
        }
    }

    pub async fn load_container(&self, cid: &str, ns: &str) -> Result<Container> {
        let namespace = self.check_namespace(ns);
        let mut c = self.client.containers();
        let request = ListContainersRequest {
            ..Default::default()
        };
        let response = c
            .list(with_namespace!(request, namespace))
            .await
            .with_context(|| format!("Failed to list containers, ns: {}", namespace))?
            .into_inner();
        let container = response
            .containers
            .into_iter()
            .find(|container| container.id == cid)
            .ok_or_else(|| anyhow!("Container {} not found", cid))?;
        Ok(container)
    }

    pub async fn get_container_list(&self, ns: &str) -> Result<Vec<String>, tonic::Status> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let mut c = self.client.containers();

        let request = ListContainersRequest {
            ..Default::default()
        };

        let request = with_namespace!(request, namespace);

        let response = c.list(request).await?;

        Ok(response
            .into_inner()
            .containers
            .into_iter()
            .map(|container| container.id)
            .collect())
    }

    pub async fn get_task(&self, cid: &str, ns: &str) -> Result<Process> {
        let namespace = self.check_namespace(ns);
        let mut tc = self.client.tasks();

        let request = ListTasksRequest {
            filter: format!("container=={}", cid),
        };

        let response = tc
            .list(with_namespace!(request, namespace))
            .await
            .with_context(|| format!("Failed to list tasks, cid: {}, ns: {}", cid, namespace))?;
        let tasks = response.into_inner().tasks;

        let task = tasks
            .into_iter()
            .find(|task| task.id == cid)
            .ok_or_else(|| anyhow!("Task for container {} not found", cid))?;

        Ok(task)
    }

    pub async fn get_task_list() {
        todo!()
    }

    async fn get_parent_snapshot(&self, img_name: &str) -> Result<String> {
        let img_config = image_manager::ImageManager::get_image_config(img_name)
            .with_context(|| format!("Failed to get image config: {}", img_name))?;

        let mut iter = img_config.rootfs().diff_ids().iter();
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

    fn check_namespace(&self, ns: &str) -> String {
        match ns {
            "" => DEFAULT_NAMESPACE.to_string(),
            _ => ns.to_string(),
        }
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let mut c = self.client.namespaces();
        let req = ListNamespacesRequest {
            ..Default::default()
        };
        let resp = c
            .list(req)
            .await
            .with_context(|| "Failed to list namespaces")?;
        Ok(resp
            .into_inner()
            .namespaces
            .into_iter()
            .map(|ns| ns.name)
            .collect())
    }

    // pub async fn get_task_list(&self, ns: &str) -> Result<Vec<String>, Err> {
    //     let mut c = self.client.tasks();
    //     let req = ListTasksRequest {
    //         ..Default::default()
    //     };
    //     let req = c.list(with_namespace!(req, ns)).await?.into_inner().tasks;
    //     Ok(())
    // }
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    netns: String,
    ip: String,
    ports: Vec<String>,
}

impl NetworkConfig {
    pub fn new(netns: String, ip: String, ports: Vec<String>) -> Self {
        NetworkConfig { netns, ip, ports }
    }

    pub fn get_netns(&self) -> String {
        self.netns.clone()
    }

    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }

    pub fn get_address(&self) -> String {
        format!(
            "{}:{}",
            self.ip.split('/').next().unwrap_or(""),
            self.ports[0].split('/').next().unwrap_or("")
        )
    }
}
