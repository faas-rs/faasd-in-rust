pub mod spec;
pub mod systemd;

use containerd_client::{
    Client,
    services::v1::{
        Container, CreateContainerRequest, CreateTaskRequest, DeleteContainerRequest,
        DeleteTaskRequest, GetImageRequest, KillRequest, ListContainersRequest,
        ListNamespacesRequest, ListTasksRequest, ReadContentRequest, StartRequest, TransferOptions,
        TransferRequest, WaitRequest,
        container::Runtime,
        snapshots::{MountsRequest, PrepareSnapshotRequest},
    },
    to_any,
    tonic::Request,
    types::{
        Mount, Platform,
        transfer::{ImageStore, OciRegistry, UnpackConfiguration},
    },
    with_namespace,
};
use oci_spec::image::{Arch, ImageConfiguration, ImageIndex, ImageManifest, MediaType, Os};
use prost_types::Any;
use sha2::{Digest, Sha256};
use spec::{DEFAULT_NAMESPACE, generate_spec};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
    time::Duration,
    vec,
};
use tokio::time::timeout;

// config.json,dockerhub密钥
// const DOCKER_CONFIG_DIR: &str = "/var/lib/faasd/.docker/";

type NetnsMap = Arc<RwLock<HashMap<String, (String, String)>>>;
lazy_static::lazy_static! {
    static ref GLOBAL_NETNS_MAP: NetnsMap = Arc::new(RwLock::new(HashMap::new()));
}

type Err = Box<dyn std::error::Error>;

pub struct Service {
    client: Arc<Client>,
    netns_map: NetnsMap,
}

impl Service {
    pub async fn new(endpoint: String) -> Result<Self, Err> {
        let client = Client::from_path(endpoint).await.unwrap();
        Ok(Service {
            client: Arc::new(client),
            netns_map: GLOBAL_NETNS_MAP.clone(),
        })
    }

    pub async fn save_netns_ip(&self, cid: &str, netns: &str, ip: &str) {
        let mut map = self.netns_map.write().unwrap();
        map.insert(cid.to_string(), (netns.to_string(), ip.to_string()));
    }

    pub async fn get_netns_ip(&self, cid: &str) -> Option<(String, String)> {
        let map = self.netns_map.read().unwrap();
        map.get(cid).cloned()
    }

    pub async fn remove_netns_ip(&self, cid: &str) {
        let mut map = self.netns_map.write().unwrap();
        map.remove(cid);
    }

    async fn prepare_snapshot(&self, cid: &str, ns: &str) -> Result<Vec<Mount>, Err> {
        let parent_snapshot = self.get_parent_snapshot(cid, ns).await?;
        let req = PrepareSnapshotRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
            parent: parent_snapshot,
            ..Default::default()
        };
        let resp = self
            .client
            .snapshots()
            .prepare(with_namespace!(req, ns))
            .await?
            .into_inner()
            .mounts;

        Ok(resp)
    }

    pub async fn create_container(&self, image_name: &str, cid: &str, ns: &str) -> Result<(), Err> {
        let namespace = match ns {
            "" => spec::DEFAULT_NAMESPACE,
            _ => ns,
        };

        let _mount = self.prepare_snapshot(cid, ns).await?;
        let (env, args) = self.get_env_and_args(image_name, ns).await?;
        let spec_path = generate_spec(cid, ns, args, env).unwrap();
        let spec = fs::read_to_string(spec_path).unwrap();

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

        // let req = with_namespace!(req, namespace);

        let _resp = containers_client
            .create(with_namespace!(req, namespace))
            .await
            .expect("Failed to create container");

        // println!("Container: {:?} created", cid);
        Ok(())
    }

    pub async fn remove_container(&self, cid: &str, ns: &str) -> Result<(), Err> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let request = ListContainersRequest {
            ..Default::default()
        };
        let mut cc = self.client.containers();

        let responce = cc
            .list(with_namespace!(request, namespace))
            .await?
            .into_inner();
        let container = responce
            .containers
            .iter()
            .find(|container| container.id == cid);

        if let Some(container) = container {
            let mut tc = self.client.tasks();

            let request = ListTasksRequest {
                filter: format!("container=={}", cid),
                ..Default::default()
            };
            let responce = tc
                .list(with_namespace!(request, namespace))
                .await?
                .into_inner();
            drop(tc);
            if let Some(task) = responce
                .tasks
                .iter()
                .find(|task| task.container_id == container.id)
            {
                println!("Task found: {}, Status: {}", task.id, task.status);
                // TASK_UNKNOWN (0) — 未知状态
                // TASK_CREATED (1) — 任务已创建
                // TASK_RUNNING (2) — 任务正在运行
                // TASK_STOPPED (3) — 任务已停止
                // TASK_EXITED (4) — 任务已退出
                // TASK_PAUSED (5) — 任务已暂停
                // TASK_FAILED (6) — 任务失败
                self.delete_task(&task.container_id, ns).await;
            }

            let delete_request = DeleteContainerRequest {
                id: container.id.clone(),
                ..Default::default()
            };

            let _ = cc
                .delete(with_namespace!(delete_request, namespace))
                .await
                .expect("Failed to delete container");
            self.remove_netns_ip(cid).await;

            // println!("Container: {:?} deleted", cc);
        } else {
            todo!("Container not found");
        }
        drop(cc);
        Ok(())
    }

    pub async fn create_and_start_task(&self, cid: &str, ns: &str) -> Result<(), Err> {
        // let tmp = std::env::temp_dir().join("containerd-client-test");
        // println!("Temp dir: {:?}", tmp);
        // fs::create_dir_all(&tmp).expect("Failed to create temp directory");
        // let stdin = tmp.join("stdin");
        // let stdout = tmp.join("stdout");
        // let stderr = tmp.join("stderr");
        // File::create(&stdin).expect("Failed to create stdin");
        // File::create(&stdout).expect("Failed to create stdout");
        // File::create(&stderr).expect("Failed to create stderr");

        let namespace = match ns {
            "" => spec::DEFAULT_NAMESPACE,
            _ => ns,
        };
        self.create_task(cid, namespace).await?;
        self.start_task(cid, namespace).await?;
        Ok(())
    }

    /// 返回任务的pid
    async fn create_task(&self, cid: &str, ns: &str) -> Result<u32, Err> {
        let mut sc = self.client.snapshots();
        let req = MountsRequest {
            snapshotter: "overlayfs".to_string(),
            key: cid.to_string(),
        };
        let mounts = sc
            .mounts(with_namespace!(req, ns))
            .await?
            .into_inner()
            .mounts;
        drop(sc);
        let (ip, path) = cni::cni_network::create_cni_network(cid.to_string(), ns.to_string())?;
        self.save_netns_ip(cid, &path, &ip).await;
        let mut tc = self.client.tasks();
        let req = CreateTaskRequest {
            container_id: cid.to_string(),
            rootfs: mounts,
            ..Default::default()
        };
        let resp = tc.create(with_namespace!(req, ns)).await?;
        let pid = resp.into_inner().pid;
        Ok(pid)
    }

    async fn start_task(&self, cid: &str, ns: &str) -> Result<(), Err> {
        let req = StartRequest {
            container_id: cid.to_string(),
            ..Default::default()
        };
        let _resp = self.client.tasks().start(with_namespace!(req, ns)).await?;

        Ok(())
    }

    pub async fn kill_task(&self, cid: String, ns: &str) -> Result<(), Err> {
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
            .expect("Failed to kill task");

        Ok(())
    }
    pub async fn pause_task() {
        todo!()
    }
    pub async fn resume_task() {
        todo!()
    }
    pub async fn delete_task(&self, cid: &str, ns: &str) {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let mut c = self.client.tasks();
        let time_out = Duration::from_secs(30);
        let wait_result = timeout(time_out, async {
            let wait_request = WaitRequest {
                container_id: cid.to_string(),
                ..Default::default()
            };

            let _ = c.wait(with_namespace!(wait_request, namespace)).await?;
            Ok::<(), Err>(())
        })
        .await;

        let kill_request = KillRequest {
            container_id: cid.to_string(),
            signal: 15,
            all: true,
            ..Default::default()
        };
        c.kill(with_namespace!(kill_request, namespace))
            .await
            .expect("Failed to kill task");

        match wait_result {
            Ok(Ok(_)) => {
                let req = DeleteTaskRequest {
                    container_id: cid.to_string(),
                };

                let _resp = c
                    .delete(with_namespace!(req, namespace))
                    .await
                    .expect("Failed to delete task");
                println!("Task: {:?} deleted", cid);
            }
            _ => {
                let kill_request = KillRequest {
                    container_id: cid.to_string(),
                    signal: 9,
                    all: true,
                    ..Default::default()
                };
                c.kill(with_namespace!(kill_request, namespace))
                    .await
                    .expect("Failed to FORCE kill task");
            }
        }
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

    pub async fn get_task_list() {
        todo!()
    }

    pub async fn prepare_image(
        &self,
        image_name: &str,
        ns: &str,
        always_pull: bool,
    ) -> Result<(), Err> {
        if always_pull {
            self.pull_image(image_name, ns).await?;
        } else {
            let namespace = self.check_namespace(ns);
            let namespace = namespace.as_str();
            let mut c = self.client.images();
            let req = GetImageRequest {
                name: image_name.to_string(),
            };
            let resp = c
                .get(with_namespace!(req, namespace))
                .await
                .map_err(|e| {
                    eprintln!(
                        "Failed to get the config of {} in namespace {}: {}",
                        image_name, namespace, e
                    );
                    e
                })
                .ok()
                .unwrap()
                .into_inner();
            if resp.image.is_none() {
                self.pull_image(image_name, ns).await?;
            }
        }
        Ok(())
    }

    pub async fn pull_image(&self, image_name: &str, ns: &str) -> Result<(), Err> {
        let namespace = self.check_namespace(ns);
        let namespace = namespace.as_str();

        let mut c = self.client.transfer();
        let source = OciRegistry {
            reference: image_name.to_string(),
            resolver: Default::default(),
        };
        // 这里先写死linux amd64
        let platform = Platform {
            os: "linux".to_string(),
            architecture: "amd64".to_string(),
            ..Default::default()
        };
        let dest = ImageStore {
            name: image_name.to_string(),
            platforms: vec![platform.clone()],
            unpacks: vec![UnpackConfiguration {
                platform: Some(platform),
                ..Default::default()
            }],
            ..Default::default()
        };

        let anys = to_any(&source);
        let anyd = to_any(&dest);

        let req = TransferRequest {
            source: Some(anys),
            destination: Some(anyd),
            options: Some(TransferOptions {
                ..Default::default()
            }),
        };
        c.transfer(with_namespace!(req, namespace))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to transfer image {} to namespace {}",
                    image_name, namespace
                )
            });
        Ok(())
    }

    // 不用这个也能拉取镜像？
    pub fn get_resolver(&self) {
        todo!()
    }

    async fn handle_index(&self, data: &Vec<u8>, ns: &str) -> Option<ImageConfiguration> {
        let image_index: ImageIndex = ::serde_json::from_slice(data).unwrap();
        let img_manifest_dscr = image_index
            .manifests()
            .iter()
            .find(|manifest_entry| match manifest_entry.platform() {
                Some(p) => {
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    {
                        matches!(p.architecture(), &Arch::Amd64) && matches!(p.os(), &Os::Linux)
                    }
                    #[cfg(target_arch = "aarch64")]
                    {
                        matches!(p.architecture(), &Arch::ARM64) && matches!(p.os(), Os::Linux)
                        //&& matches!(p.variant().as_ref().map(|s| s.as_str()), Some("v8"))
                    }
                }
                None => false,
            })
            .unwrap();

        let req = ReadContentRequest {
            digest: img_manifest_dscr.digest().to_owned(),
            offset: 0,
            size: 0,
        };

        let mut c = self.client.content();
        let resp = c
            .read(with_namespace!(req, ns))
            .await
            .expect("Failed to read content")
            .into_inner()
            .message()
            .await
            .expect("Failed to read content message")
            .unwrap()
            .data;

        self.handle_manifest(&resp, ns).await
    }

    async fn handle_manifest(&self, data: &Vec<u8>, ns: &str) -> Option<ImageConfiguration> {
        let img_manifest: ImageManifest = ::serde_json::from_slice(data).unwrap();
        let img_manifest_dscr = img_manifest.config();

        let req = ReadContentRequest {
            digest: img_manifest_dscr.digest().to_owned(),
            offset: 0,
            size: 0,
        };
        let mut c = self.client.content();

        let resp = c
            .read(with_namespace!(req, ns))
            .await
            .unwrap()
            .into_inner()
            .message()
            .await
            .unwrap()
            .unwrap()
            .data;

        ::serde_json::from_slice(&resp).unwrap()
    }

    pub async fn get_img_config(&self, name: &str, ns: &str) -> Option<ImageConfiguration> {
        let mut c = self.client.images();

        let req = GetImageRequest {
            name: name.to_string(),
        };
        let resp = c
            .get(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                eprintln!(
                    "Failed to get the config of {} in namespace {}: {}",
                    name, ns, e
                );
                e
            })
            .ok()?
            .into_inner();

        let img_dscr = resp.image?.target?;
        let media_type = MediaType::from(img_dscr.media_type.as_str());

        let req = ReadContentRequest {
            digest: img_dscr.digest,
            ..Default::default()
        };

        let mut c = self.client.content();

        let resp = c
            .read(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                eprintln!(
                    "Failed to read content for {} in namespace {}: {}",
                    name, ns, e
                );
                e
            })
            .ok()?
            .into_inner()
            .message()
            .await
            .map_err(|e| {
                eprintln!(
                    "Failed to read message for {} in namespace {}: {}",
                    name, ns, e
                );
                e
            })
            .ok()?
            .ok_or_else(|| {
                eprintln!("No data found for {} in namespace {}", name, ns);
                std::io::Error::new(std::io::ErrorKind::NotFound, "No data found")
            })
            .ok()?
            .data;

        let img_config = match media_type {
            MediaType::ImageIndex => self.handle_index(&resp, ns).await.unwrap(),
            MediaType::ImageManifest => self.handle_manifest(&resp, ns).await.unwrap(),
            MediaType::Other(media_type) => match media_type.as_str() {
                "application/vnd.docker.distribution.manifest.list.v2+json" => {
                    self.handle_index(&resp, ns).await.unwrap()
                }
                "application/vnd.docker.distribution.manifest.v2+json" => {
                    self.handle_manifest(&resp, ns).await.unwrap()
                }
                _ => {
                    eprintln!("Unexpected media type '{}'", media_type);
                    return None;
                }
            },
            _ => {
                eprintln!("Unexpected media type '{}'", media_type);
                return None;
            }
        };
        Some(img_config)
    }

    async fn get_parent_snapshot(&self, name: &str, ns: &str) -> Result<String, Err> {
        let img_config = self.get_img_config(name, ns).await.unwrap();

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

    async fn get_env_and_args(
        &self,
        name: &str,
        ns: &str,
    ) -> Result<(Vec<String>, Vec<String>), Err> {
        let img_config = self.get_img_config(name, ns).await.unwrap();
        if let Some(config) = img_config.config() {
            let env = config.env().as_ref().map_or_else(Vec::new, |v| v.clone());
            let args = config.cmd().as_ref().map_or_else(Vec::new, |v| v.clone());
            Ok((env, args))
        } else {
            Err("No config found".into())
        }
    }

    fn check_namespace(&self, ns: &str) -> String {
        match ns {
            "" => DEFAULT_NAMESPACE.to_string(),
            _ => ns.to_string(),
        }
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>, Err> {
        let mut c = self.client.namespaces();
        let req = ListNamespacesRequest {
            ..Default::default()
        };
        let resp = c.list(req).await?;
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
//容器是容器，要先启动，然后才能运行任务
//要想删除一个正在运行的Task，必须先kill掉这个task，然后才能删除。
