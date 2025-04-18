use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use containerd_client::{
    Client,
    services::v1::{GetImageRequest, ReadContentRequest, TransferOptions, TransferRequest},
    to_any,
    tonic::Request,
    types::{
        Platform,
        transfer::{ImageStore, OciRegistry, UnpackConfiguration},
    },
    with_namespace,
};
use oci_spec::image::{Arch, ImageConfiguration, ImageIndex, ImageManifest, MediaType, Os};

use crate::{containerd_manager::CLIENT, spec::DEFAULT_NAMESPACE};

type ImagesMap = Arc<RwLock<HashMap<String, ImageConfiguration>>>;
lazy_static::lazy_static! {
    static ref GLOBAL_IMAGE_MAP: ImagesMap = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug, Clone)]
pub struct ImageRuntimeConfig {
    pub env: Vec<String>,
    pub args: Vec<String>,
    pub ports: Vec<String>,
    pub cwd: String,
}

impl ImageRuntimeConfig {
    pub fn new(env: Vec<String>, args: Vec<String>, ports: Vec<String>, cwd: String) -> Self {
        ImageRuntimeConfig {
            env,
            args,
            ports,
            cwd,
        }
    }
}

impl Drop for ImageManager {
    fn drop(&mut self) {
        let mut map = GLOBAL_IMAGE_MAP.write().unwrap();
        map.clear();
    }
}

#[derive(Debug)]
pub enum ImageError {
    ImageNotFound(String),
    ImagePullFailed(String),
    ImageConfigurationNotFound(String),
    ReadContentFailed(String),
    UnexpectedMediaType,
    DeserializationFailed(String),
    #[allow(dead_code)]
    OtherError,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::ImageNotFound(msg) => write!(f, "Image not found: {}", msg),
            ImageError::ImagePullFailed(msg) => write!(f, "Image pull failed: {}", msg),
            ImageError::ImageConfigurationNotFound(msg) => {
                write!(f, "Image configuration not found: {}", msg)
            }
            ImageError::ReadContentFailed(msg) => write!(f, "Read content failed: {}", msg),
            ImageError::UnexpectedMediaType => {
                write!(f, "Unexpected media type")
            }
            ImageError::DeserializationFailed(msg) => {
                write!(f, "Deserialization failed: {}", msg)
            }
            ImageError::OtherError => write!(f, "Other error happened"),
        }
    }
}

impl std::error::Error for ImageError {}

#[derive(Debug)]
pub struct ImageManager;

impl ImageManager {
    async fn get_client() -> Arc<Client> {
        CLIENT
            .get()
            .unwrap_or_else(|| panic!("Client not initialized, Please run init first"))
            .clone()
    }

    pub async fn prepare_image(
        image_name: &str,
        ns: &str,
        always_pull: bool,
    ) -> Result<(), ImageError> {
        if always_pull {
            Self::pull_image(image_name, ns).await?;
        } else {
            let namespace = check_namespace(ns);
            let namespace = namespace.as_str();

            Self::get_image(image_name, namespace).await?;
        }
        Self::save_img_config(image_name, ns).await
    }

    async fn get_image(image_name: &str, ns: &str) -> Result<(), ImageError> {
        let mut c = Self::get_client().await.images();
        let req = GetImageRequest {
            name: image_name.to_string(),
        };

        let resp = match c.get(with_namespace!(req, ns)).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                return Err(ImageError::ImageNotFound(format!(
                    "Failed to get image {}: {}",
                    image_name, e
                )));
            }
        };
        if resp.image.is_none() {
            Self::pull_image(image_name, ns).await?;
        }
        Ok(())
    }

    pub async fn pull_image(image_name: &str, ns: &str) -> Result<(), ImageError> {
        let client = Self::get_client().await;
        let ns = check_namespace(ns);
        let namespace = ns.as_str();

        let mut c: containerd_client::services::v1::transfer_client::TransferClient<
            tonic::transport::Channel,
        > = client.transfer();
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

        if let Err(e) = c.transfer(with_namespace!(req, namespace)).await {
            return Err(ImageError::ImagePullFailed(format!(
                "Failed to pull image {}: {}",
                image_name, e
            )));
        }

        Ok(())
        // Self::save_img_config(client, image_name, ns.as_str()).await
    }

    pub async fn save_img_config(img_name: &str, ns: &str) -> Result<(), ImageError> {
        let client = Self::get_client().await;
        let mut c = client.images();

        let req = GetImageRequest {
            name: img_name.to_string(),
        };
        let resp = match c.get(with_namespace!(req, ns)).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                return Err(ImageError::ImageNotFound(format!(
                    "Failed to get image {}: {}",
                    img_name, e
                )));
            }
        };

        let img_dscr = resp.image.unwrap().target.unwrap();
        let media_type = MediaType::from(img_dscr.media_type.as_str());

        let req = ReadContentRequest {
            digest: img_dscr.digest,
            ..Default::default()
        };

        let mut c = client.content();

        let mut inner = match c.read(with_namespace!(req, ns)).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to read content of image {}: {}",
                    img_name, e
                )));
            }
        };

        let resp = match inner.message().await {
            Ok(response) => response.unwrap().data,
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to get the inner content of image {}: {}",
                    img_name, e
                )));
            }
        };

        drop(c);

        let img_config = match media_type {
            MediaType::ImageIndex => Self::handle_index(&resp, ns).await.unwrap(),
            MediaType::ImageManifest => Self::handle_manifest(&resp, ns).await.unwrap(),
            MediaType::Other(media_type) => match media_type.as_str() {
                "application/vnd.docker.distribution.manifest.list.v2+json" => {
                    Self::handle_index(&resp, ns).await.unwrap()
                }
                "application/vnd.docker.distribution.manifest.v2+json" => {
                    Self::handle_manifest(&resp, ns).await.unwrap()
                }
                _ => {
                    return Err(ImageError::UnexpectedMediaType);
                }
            },
            _ => {
                return Err(ImageError::UnexpectedMediaType);
            }
        };
        if img_config.is_none() {
            return Err(ImageError::ImageConfigurationNotFound(format!(
                "save_img_config: Image configuration not found for image {}",
                img_name
            )));
        }
        let img_config = img_config.unwrap();
        Self::insert_image_config(img_name, img_config)
    }

    async fn handle_index(data: &[u8], ns: &str) -> Result<Option<ImageConfiguration>, ImageError> {
        let image_index: ImageIndex = ::serde_json::from_slice(data).map_err(|e| {
            ImageError::DeserializationFailed(format!("Failed to parse JSON: {}", e))
        })?;
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

        let mut c = Self::get_client().await.content();
        let mut inner = match c.read(with_namespace!(req, ns)).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to handler index : {}",
                    e
                )));
            }
        };

        let resp = match inner.message().await {
            Ok(response) => response.unwrap().data,
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to handle index inner : {}",
                    e
                )));
            }
        };
        drop(c);

        Self::handle_manifest(&resp, ns).await
    }

    async fn handle_manifest(
        data: &[u8],
        ns: &str,
    ) -> Result<Option<ImageConfiguration>, ImageError> {
        let img_manifest: ImageManifest = match ::serde_json::from_slice(data) {
            Ok(manifest) => manifest,
            Err(e) => {
                return Err(ImageError::DeserializationFailed(format!(
                    "Failed to deserialize image manifest: {}",
                    e
                )));
            }
        };
        let img_manifest_dscr = img_manifest.config();

        let req = ReadContentRequest {
            digest: img_manifest_dscr.digest().to_owned(),
            offset: 0,
            size: 0,
        };
        let mut c = Self::get_client().await.content();

        let mut inner = match c.read(with_namespace!(req, ns)).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to handler index : {}",
                    e
                )));
            }
        };

        let resp = match inner.message().await {
            Ok(response) => response.unwrap().data,
            Err(e) => {
                return Err(ImageError::ReadContentFailed(format!(
                    "Failed to handle index inner : {}",
                    e
                )));
            }
        };

        Ok(::serde_json::from_slice(&resp).unwrap())
    }

    fn insert_image_config(image_name: &str, config: ImageConfiguration) -> Result<(), ImageError> {
        let mut map = GLOBAL_IMAGE_MAP.write().unwrap();
        map.insert(image_name.to_string(), config);
        Ok(())
    }

    pub fn get_image_config(image_name: &str) -> Result<ImageConfiguration, ImageError> {
        let map = GLOBAL_IMAGE_MAP.read().unwrap();
        if let Some(config) = map.get(image_name) {
            Ok(config.clone())
        } else {
            Err(ImageError::ImageConfigurationNotFound(format!(
                "get_image_config: Image configuration not found for image {}",
                image_name
            )))
        }
    }

    pub fn get_runtime_config(image_name: &str) -> Result<ImageRuntimeConfig, ImageError> {
        let map = GLOBAL_IMAGE_MAP.read().unwrap();
        if let Some(config) = map.get(image_name) {
            if let Some(config) = config.config() {
                let env = config
                    .env()
                    .clone()
                    .expect("Failed to get environment variables");
                let args = config
                    .cmd()
                    .clone()
                    .expect("Failed to get command arguments");
                let ports = config.exposed_ports().clone().unwrap_or_else(|| {
                    log::warn!("Exposed ports not found, using default port 8080/tcp");
                    vec!["8080/tcp".to_string()]
                });
                let cwd = config.working_dir().clone().unwrap_or_else(|| {
                    log::warn!("Working directory not found, using default /");
                    "/".to_string()
                });
                Ok(ImageRuntimeConfig::new(env, args, ports, cwd))
            } else {
                Err(ImageError::ImageConfigurationNotFound(format!(
                    "Image configuration is empty for image {}",
                    image_name
                )))
            }
        } else {
            Err(ImageError::ImageConfigurationNotFound(format!(
                "get_runtime_config: Image configuration not found for image {}",
                image_name
            )))
        }
    }

    // 不用这个也能拉取镜像？
    pub fn get_resolver() {
        todo!()
    }
}

fn check_namespace(ns: &str) -> String {
    match ns {
        "" => DEFAULT_NAMESPACE.to_string(),
        _ => ns.to_string(),
    }
}
