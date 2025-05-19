use super::ContainerdService;

use container_image_dist_ref::ImgRef;
use containerd_client::{
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

impl ContainerdService {
    async fn get_image(&self, image_name: &str, ns: &str) -> Result<(), ImageError> {
        let mut c = self.client.images();
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
            self.pull_image(image_name, ns).await?;
        }
        Ok(())
    }

    pub async fn pull_image(&self, image_name: &str, ns: &str) -> Result<(), ImageError> {
        let ns = check_namespace(ns);
        let namespace = ns.as_str();

        let mut trans_cli = self.client.transfer();
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

        trans_cli
            .transfer(with_namespace!(req, namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to pull image: {}", e);
                ImageError::ImagePullFailed(format!("Failed to pull image {}: {}", image_name, e))
            })
            .map(|resp| {
                log::trace!("Pull image response: {:?}", resp);
            })
    }

    pub async fn prepare_image(
        &self,
        image_name: &str,
        ns: &str,
        always_pull: bool,
    ) -> Result<(), ImageError> {
        let _ = ImgRef::new(image_name).map_err(|e| {
            ImageError::ImageNotFound(format!("Invalid image name: {:?}", e.kind()))
        })?;
        if always_pull {
            self.pull_image(image_name, ns).await
        } else {
            let namespace = check_namespace(ns);
            let namespace = namespace.as_str();

            self.get_image(image_name, namespace).await
        }
    }

    pub async fn image_config(
        &self,
        img_name: &str,
        ns: &str,
    ) -> Result<ImageConfiguration, ImageError> {
        let mut img_cli = self.client.images();

        let req = GetImageRequest {
            name: img_name.to_string(),
        };
        let resp = match img_cli.get(with_namespace!(req, ns)).await {
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

        let mut content_cli = self.client.content();

        let mut inner = match content_cli.read(with_namespace!(req, ns)).await {
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

        drop(content_cli);

        match media_type {
            MediaType::ImageIndex => self.handle_index(&resp, ns).await,
            MediaType::ImageManifest => self.handle_manifest(&resp, ns).await,
            MediaType::Other(val)
                if val == "application/vnd.docker.distribution.manifest.list.v2+json" =>
            {
                self.handle_index(&resp, ns).await
            }
            MediaType::Other(val)
                if val == "application/vnd.docker.distribution.manifest.v2+json" =>
            {
                self.handle_manifest(&resp, ns).await
            }
            _ => Err(ImageError::UnexpectedMediaType),
        }
    }

    async fn handle_index(&self, data: &[u8], ns: &str) -> Result<ImageConfiguration, ImageError> {
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

        let mut c = self.client.content();
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

        self.handle_manifest(&resp, ns).await
    }

    async fn handle_manifest(
        &self,
        data: &[u8],
        ns: &str,
    ) -> Result<ImageConfiguration, ImageError> {
        let img_manifest: ImageManifest = match serde_json::from_slice(data) {
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
        let mut c = self.client.content();

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

        serde_json::from_slice(&resp)
            .map_err(|e| ImageError::DeserializationFailed(format!("Failed to parse JSON: {}", e)))
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

// 不用这个也能拉取镜像？
pub fn get_resolver() {
    todo!()
}

fn check_namespace(ns: &str) -> String {
    match ns {
        "" => crate::consts::DEFAULT_FUNCTION_NAMESPACE.to_string(),
        _ => ns.to_string(),
    }
}
