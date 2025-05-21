use super::{
    ContainerdService, cni::Endpoint, error::ContainerdError, function::ContainerStaticMetadata,
};
use crate::consts::{VERSION_DEV, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};
use oci_spec::{
    image::ImageConfiguration,
    runtime::{
        Capability, LinuxBuilder, LinuxCapabilitiesBuilder, LinuxDeviceCgroupBuilder,
        LinuxNamespaceBuilder, LinuxNamespaceType, LinuxResourcesBuilder, MountBuilder,
        PosixRlimitBuilder, PosixRlimitType, ProcessBuilder, RootBuilder, Spec, SpecBuilder,
        UserBuilder,
    },
};
use std::path::Path;

fn oci_version() -> String {
    format!(
        "{}.{}.{}{}",
        VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, VERSION_DEV
    )
}

pub(super) fn generate_default_unix_spec(
    ns: &str,
    cid: &str,
    runtime_config: &RuntimeConfig,
) -> Result<oci_spec::runtime::Spec, ContainerdError> {
    let caps = [
        Capability::Chown,
        Capability::DacOverride,
        Capability::Fsetid,
        Capability::Fowner,
        Capability::Mknod,
        Capability::NetRaw,
        Capability::Setgid,
        Capability::Setuid,
        Capability::Setfcap,
        Capability::Setpcap,
        Capability::NetBindService,
        Capability::SysChroot,
        Capability::Kill,
        Capability::AuditWrite,
    ];
    let spec = SpecBuilder::default()
        .version(oci_version())
        .root(
            RootBuilder::default()
                .path("rootfs")
                .readonly(true)
                .build()
                .unwrap(),
        )
        .process(
            ProcessBuilder::default()
                .cwd(runtime_config.cwd.clone())
                .no_new_privileges(true)
                .user(UserBuilder::default().uid(0u32).gid(0u32).build().unwrap())
                .capabilities(
                    LinuxCapabilitiesBuilder::default()
                        .bounding(caps)
                        .permitted(caps)
                        .effective(caps)
                        .build()
                        .unwrap(),
                )
                .rlimits([PosixRlimitBuilder::default()
                    .typ(PosixRlimitType::RlimitNofile)
                    .hard(1024u64)
                    .soft(1024u64)
                    .build()
                    .unwrap()])
                .args(runtime_config.args.clone())
                .env(runtime_config.env.clone())
                .build()
                .unwrap(),
        )
        .linux(
            LinuxBuilder::default()
                .masked_paths([
                    "/proc/acpi".into(),
                    "/proc/asound".into(),
                    "/proc/kcore".into(),
                    "/proc/keys".into(),
                    "/proc/latency_stats".into(),
                    "/proc/timer_list".into(),
                    "/proc/timer_stats".into(),
                    "/proc/sched_debug".into(),
                    "/sys/firmware".into(),
                    "/proc/scsi".into(),
                    "/sys/devices/virtual/powercap".into(),
                ])
                .readonly_paths([
                    "/proc/bus".into(),
                    "/proc/fs".into(),
                    "/proc/irq".into(),
                    "/proc/sys".into(),
                    "/proc/sysrq-trigger".into(),
                ])
                .cgroups_path(Path::new("/").join(ns).join(cid))
                .resources(
                    LinuxResourcesBuilder::default()
                        .devices([LinuxDeviceCgroupBuilder::default()
                            .allow(false)
                            .access("rwm")
                            .build()
                            .unwrap()])
                        .build()
                        .unwrap(),
                )
                .namespaces([
                    LinuxNamespaceBuilder::default()
                        .typ(LinuxNamespaceType::Pid)
                        .build()
                        .unwrap(),
                    LinuxNamespaceBuilder::default()
                        .typ(LinuxNamespaceType::Ipc)
                        .build()
                        .unwrap(),
                    LinuxNamespaceBuilder::default()
                        .typ(LinuxNamespaceType::Uts)
                        .build()
                        .unwrap(),
                    LinuxNamespaceBuilder::default()
                        .typ(LinuxNamespaceType::Mount)
                        .build()
                        .unwrap(),
                    LinuxNamespaceBuilder::default()
                        .typ(LinuxNamespaceType::Network)
                        .path(format!("/var/run/netns/{}", Endpoint::new(cid, ns)))
                        .build()
                        .unwrap(),
                ])
                .build()
                .unwrap(),
        )
        .mounts([
            MountBuilder::default()
                .destination("/proc")
                .typ("proc")
                .source("proc")
                .options(["nosuid".into(), "noexec".into(), "nodev".into()])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/dev")
                .typ("tmpfs")
                .source("tmpfs")
                .options([
                    "nosuid".into(),
                    "strictatime".into(),
                    "mode=755".into(),
                    "size=65536k".into(),
                ])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/dev/pts")
                .typ("devpts")
                .source("devpts")
                .options([
                    "nosuid".into(),
                    "noexec".into(),
                    "newinstance".into(),
                    "ptmxmode=0666".into(),
                    "mode=0620".into(),
                    "gid=5".into(),
                ])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/dev/shm")
                .typ("tmpfs")
                .source("shm")
                .options([
                    "nosuid".into(),
                    "noexec".into(),
                    "nodev".into(),
                    "mode=1777".into(),
                    "size=65536k".into(),
                ])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/dev/mqueue")
                .typ("mqueue")
                .source("mqueue")
                .options(["nosuid".into(), "noexec".into(), "nodev".into()])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/sys")
                .typ("sysfs")
                .source("sysfs")
                .options([
                    "nosuid".into(),
                    "noexec".into(),
                    "nodev".into(),
                    "ro".into(),
                ])
                .build()
                .unwrap(),
            MountBuilder::default()
                .destination("/run")
                .typ("tmpfs")
                .source("tmpfs")
                .options([
                    "nosuid".into(),
                    "strictatime".into(),
                    "mode=755".into(),
                    "size=65536k".into(),
                ])
                .build()
                .unwrap(),
        ])
        .build()
        .map_err(|e| {
            log::error!("Failed to generate spec: {}", e);
            ContainerdError::GenerateSpecError(e.to_string())
        })?;

    Ok(spec)
}

#[allow(unused)]
pub(super) fn with_vm_network(spec: &mut Spec) -> Result<(), ContainerdError> {
    let mounts = spec
        .mounts()
        .as_ref()
        .expect("Spec's 'Mounts' field should not be None");
    let mut new_mounts = mounts.clone();
    new_mounts.extend([
        MountBuilder::default()
            .destination("/etc/resolv.conf")
            .typ("bind")
            .source("/etc/resolv.conf")
            .options(["rbind".into(), "ro".into()])
            .build()
            .map_err(|e| {
                log::error!("Failed to build OCI (resolv.conf) Mount: {}", e);
                ContainerdError::GenerateSpecError(e.to_string())
            })?,
        MountBuilder::default()
            .destination("/etc/hosts")
            .typ("bind")
            .source("/etc/hosts")
            .options(["rbind".into(), "ro".into()])
            .build()
            .map_err(|e| {
                log::error!("Failed to build OCI (hosts) Mount: {}", e);
                ContainerdError::GenerateSpecError(e.to_string())
            })?,
    ]);
    let _ = spec.set_mounts(Some(new_mounts));

    Ok(())
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub env: Vec<String>,
    pub args: Vec<String>,
    pub ports: Vec<String>,
    pub cwd: String,
}

impl TryFrom<ImageConfiguration> for RuntimeConfig {
    type Error = ContainerdError;

    fn try_from(value: ImageConfiguration) -> Result<Self, Self::Error> {
        let conf_ref = value.config().as_ref();
        let config = conf_ref.ok_or(ContainerdError::GenerateSpecError(
            "Image configuration not found".to_string(),
        ))?;

        let env = config.env().clone().ok_or_else(|| {
            ContainerdError::GenerateSpecError("Environment variables not found".to_string())
        })?;
        let args = config.cmd().clone().ok_or_else(|| {
            ContainerdError::GenerateSpecError("Command arguments not found".to_string())
        })?;
        let ports = config.exposed_ports().clone().unwrap_or_else(|| {
            log::warn!("Exposed ports not found, using default port 8080/tcp");
            vec!["8080/tcp".to_string()]
        });
        let cwd = config.working_dir().clone().unwrap_or_else(|| {
            log::warn!("Working directory not found, using default /");
            "/".to_string()
        });
        Ok(RuntimeConfig {
            env,
            args,
            ports,
            cwd,
        })
    }
}

impl ContainerdService {
    pub async fn get_spec(
        &self,
        metadata: &ContainerStaticMetadata,
    ) -> Result<prost_types::Any, ContainerdError> {
        let image_conf = self
            .image_config(&metadata.image, &metadata.endpoint.namespace)
            .await
            .map_err(|e| {
                log::error!("Failed to get image config: {}", e);
                ContainerdError::GenerateSpecError(e.to_string())
            })?;

        let rt_conf = RuntimeConfig::try_from(image_conf)?;

        let spec = generate_default_unix_spec(
            &metadata.endpoint.namespace,
            &metadata.endpoint.service,
            &rt_conf,
        )?;
        let spec_json = serde_json::to_string(&spec).map_err(|e| {
            log::error!("Failed to serialize spec to JSON: {}", e);
            ContainerdError::GenerateSpecError(e.to_string())
        })?;
        let any_spec = prost_types::Any {
            type_url: "types.containerd.io/opencontainers/runtime-spec/1/Spec".to_string(),
            value: spec_json.into_bytes(),
        };

        Ok(any_spec)
    }
}
