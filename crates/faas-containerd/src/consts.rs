#[allow(unused)]
pub const DEFAULT_FUNCTION_NAMESPACE: &str = "faasrs-default";

#[allow(unused)]
pub const DEFAULT_SNAPSHOTTER: &str = "overlayfs";

pub const DEFAULT_CTRD_SOCK: &str = "/run/containerd/containerd.sock";

pub const DEFAULT_FAASDRS_DATA_DIR: &str = "/var/lib/faasdrs";

// 定义版本的常量
pub const VERSION_MAJOR: u32 = 1;
pub const VERSION_MINOR: u32 = 1;
pub const VERSION_PATCH: u32 = 0;
pub const VERSION_DEV: &str = ""; // 对应开发分支
