use serde::{Deserialize, Serialize};
use std::fs::File;

// 定义版本的常量
const VERSION_MAJOR: u32 = 1;
const VERSION_MINOR: u32 = 1;
const VERSION_PATCH: u32 = 0;
const VERSION_DEV: &str = ""; // 对应开发分支

const RWM: &str = "rwm";
const DEFAULT_ROOTFS_PATH: &str = "rootfs";
pub const DEFAULT_NAMESPACE: &str = "default";
const PATH_TO_SPEC_PREFIX: &str = "/tmp/containerd-spec";

// const DEFAULT_UNIX_ENV: &str = "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

const PID_NAMESPACE: &str = "pid";
const NETWORK_NAMESPACE: &str = "network";
const MOUNT_NAMESPACE: &str = "mount";
const IPC_NAMESPACE: &str = "ipc";
const UTS_NAMESPACE: &str = "uts";

#[derive(Serialize, Deserialize, Debug)]
pub struct Spec {
    #[serde(rename = "ociVersion")]
    pub oci_version: String,
    pub root: Root,
    pub process: Process,
    pub linux: Linux,
    pub mounts: Vec<Mount>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Root {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Process {
    pub cwd: String,
    #[serde(rename = "noNewPrivileges")]
    pub no_new_privileges: bool,
    pub user: User,
    pub capabilities: LinuxCapabilities,
    pub rlimits: Vec<POSIXRlimit>,
    pub args: Vec<String>,
    pub env: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub uid: u32,
    pub gid: u32,
    #[serde(rename = "additionalGids")]
    pub additional_gids: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mount {
    pub destination: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub source: String,
    pub options: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinuxCapabilities {
    pub bounding: Vec<String>,
    pub permitted: Vec<String>,
    pub effective: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct POSIXRlimit {
    pub hard: u64,
    pub soft: u64,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Linux {
    pub masked_paths: Vec<String>,
    pub readonly_paths: Vec<String>,
    pub cgroups_path: String,
    pub resources: LinuxResources,
    pub namespaces: Vec<LinuxNamespace>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinuxResources {
    pub devices: Vec<LinuxDeviceCgroup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinuxDeviceCgroup {
    pub allow: bool,
    pub access: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinuxNamespace {
    #[serde(rename = "type")]
    pub type_: String,
    pub path: Option<String>,
}

pub fn default_unix_caps() -> Vec<String> {
    vec![
        String::from("CAP_CHOWN"),
        String::from("CAP_DAC_OVERRIDE"),
        String::from("CAP_FSETID"),
        String::from("CAP_FOWNER"),
        String::from("CAP_MKNOD"),
        String::from("CAP_NET_RAW"),
        String::from("CAP_SETGID"),
        String::from("CAP_SETUID"),
        String::from("CAP_SETFCAP"),
        String::from("CAP_SETPCAP"),
        String::from("CAP_NET_BIND_SERVICE"),
        String::from("CAP_SYS_CHROOT"),
        String::from("CAP_KILL"),
        String::from("CAP_AUDIT_WRITE"),
    ]
}

fn default_masked_parhs() -> Vec<String> {
    vec![
        String::from("/proc/acpi"),
        String::from("/proc/asound"),
        String::from("/proc/kcore"),
        String::from("/proc/keys"),
        String::from("/proc/latency_stats"),
        String::from("/proc/timer_list"),
        String::from("/proc/timer_stats"),
        String::from("/proc/sched_debug"),
        String::from("/proc/scsi"),
        String::from("/sys/firmware"),
        String::from("/sys/devices/virtual/powercap"),
    ]
}

fn default_readonly_paths() -> Vec<String> {
    vec![
        String::from("/proc/bus"),
        String::from("/proc/fs"),
        String::from("/proc/irq"),
        String::from("/proc/sys"),
        String::from("/proc/sysrq-trigger"),
    ]
}

fn default_unix_namespaces(ns: &str, cid: &str) -> Vec<LinuxNamespace> {
    vec![
        LinuxNamespace {
            type_: String::from(PID_NAMESPACE),
            path: None,
        },
        LinuxNamespace {
            type_: String::from(IPC_NAMESPACE),
            path: None,
        },
        LinuxNamespace {
            type_: String::from(UTS_NAMESPACE),
            path: None,
        },
        LinuxNamespace {
            type_: String::from(MOUNT_NAMESPACE),
            path: None,
        },
        LinuxNamespace {
            type_: String::from(NETWORK_NAMESPACE),
            path: Some(format!("/var/run/netns/{}", get_netns(ns, cid))),
        },
    ]
}

fn default_mounts() -> Vec<Mount> {
    vec![
        Mount {
            destination: "/proc".to_string(),
            type_: "proc".to_string(),
            source: "proc".to_string(),
            options: vec![],
        },
        Mount {
            destination: "/dev".to_string(),
            type_: "tmpfs".to_string(),
            source: "tmpfs".to_string(),
            options: vec![
                "nosuid".to_string(),
                "strictatime".to_string(),
                "mode=755".to_string(),
                "size=65536k".to_string(),
            ],
        },
        Mount {
            destination: "/dev/pts".to_string(),
            type_: "devpts".to_string(),
            source: "devpts".to_string(),
            options: vec![
                "nosuid".to_string(),
                "noexec".to_string(),
                "newinstance".to_string(),
                "ptmxmode=0666".to_string(),
                "mode=0620".to_string(),
                "gid=5".to_string(),
            ],
        },
        Mount {
            destination: "/dev/shm".to_string(),
            type_: "tmpfs".to_string(),
            source: "shm".to_string(),
            options: vec![
                "nosuid".to_string(),
                "noexec".to_string(),
                "nodev".to_string(),
                "mode=1777".to_string(),
                "size=65536k".to_string(),
            ],
        },
        Mount {
            destination: "/dev/mqueue".to_string(),
            type_: "mqueue".to_string(),
            source: "mqueue".to_string(),
            options: vec![
                "nosuid".to_string(),
                "noexec".to_string(),
                "nodev".to_string(),
            ],
        },
        Mount {
            destination: "/sys".to_string(),
            type_: "sysfs".to_string(),
            source: "sysfs".to_string(),
            options: vec![
                "nosuid".to_string(),
                "noexec".to_string(),
                "nodev".to_string(),
                "ro".to_string(),
            ],
        },
        Mount {
            destination: "/sys/fs/cgroup".to_string(),
            type_: "cgroup".to_string(),
            source: "cgroup".to_string(),
            options: vec![
                "nosuid".to_string(),
                "noexec".to_string(),
                "nodev".to_string(),
                "relatime".to_string(),
                "ro".to_string(),
            ],
        },
    ]
}

fn get_version() -> String {
    format!(
        "{}.{}.{}{}",
        VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, VERSION_DEV
    )
}

pub fn populate_default_unix_spec(id: &str, ns: &str) -> Spec {
    Spec {
        oci_version: get_version(),
        root: Root {
            path: DEFAULT_ROOTFS_PATH.to_string(),
        },
        process: Process {
            cwd: String::from("/"),
            no_new_privileges: true,
            user: User {
                uid: 0,
                gid: 0,
                additional_gids: vec![],
            },
            capabilities: LinuxCapabilities {
                bounding: default_unix_caps(),
                permitted: default_unix_caps(),
                effective: default_unix_caps(),
            },
            rlimits: vec![POSIXRlimit {
                type_: String::from("RLIMIT_NOFILE"),
                hard: 1024,
                soft: 1024,
            }],
            args: vec![],
            env: vec![],
        },
        linux: Linux {
            masked_paths: default_masked_parhs(),
            readonly_paths: default_readonly_paths(),
            cgroups_path: format!("{}/{}", ns, id),
            resources: LinuxResources {
                devices: vec![LinuxDeviceCgroup {
                    allow: false,
                    access: RWM.to_string(),
                }],
            },
            namespaces: default_unix_namespaces(ns, id),
        },
        mounts: default_mounts(),
    }
}

pub fn save_spec_to_file(spec: &Spec, path: &str) -> Result<(), std::io::Error> {
    let file = File::create(path)?;
    serde_json::to_writer(file, spec)?;
    Ok(())
}

fn get_netns(ns: &str, cid: &str) -> String {
    format!("{}-{}", ns, cid)
}

pub fn generate_spec(
    id: &str,
    ns: &str,
    args: Vec<String>,
    env: Vec<String>,
) -> Result<String, std::io::Error> {
    let namespace = match ns {
        "" => DEFAULT_NAMESPACE,
        _ => ns,
    };
    let mut spec = populate_default_unix_spec(id, ns);
    spec.process.args = args;
    spec.process.env = env;
    let path = format!("{}/{}/{}.json", PATH_TO_SPEC_PREFIX, namespace, id);
    save_spec_to_file(&spec, &path)?;
    Ok(path)
}
