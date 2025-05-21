use std::{
    io::Error,
    path::Path,
    process::{Command, Output},
    sync::LazyLock,
};

static CNI_BIN_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("CNI_BIN_DIR").unwrap_or_else(|_| "/opt/cni/bin".to_string()));
static CNI_TOOL: LazyLock<String> =
    LazyLock::new(|| std::env::var("CNI_TOOL").unwrap_or_else(|_| "cni-tool".to_string()));

pub(super) fn cni_add_bridge(
    netns_path: &Path,
    bridge_network_name: &str,
) -> Result<Output, Error> {
    Command::new(CNI_TOOL.as_str())
        .arg("add")
        .arg(bridge_network_name)
        .arg(netns_path)
        .env("CNI_PATH", CNI_BIN_DIR.as_str())
        .output()
}

pub(super) fn cni_del_bridge(
    netns_path: &Path,
    bridge_network_name: &str,
) -> Result<Output, Error> {
    Command::new(CNI_TOOL.as_str())
        .arg("del")
        .arg(bridge_network_name)
        .arg(netns_path)
        .env("CNI_PATH", CNI_BIN_DIR.as_str())
        .output()
}

// /// THESE TESTS SHOULD BE RUN WITH ROOT PRIVILEGES
// #[cfg(test)]
// mod test {
//     use crate::impls::cni::util;
//     use std::path::Path;

//     use super::*;

//     const CNI_DATA_DIR: &str = "/var/run/cni";
//     const TEST_CNI_CONF_FILENAME: &str = "11-faasrstest.conflist";
//     const TEST_NETWORK_NAME: &str = "faasrstest-cni-bridge";
//     const TEST_BRIDGE_NAME: &str = "faasrstest0";
//     const TEST_SUBNET: &str = "10.99.0.0/16";
//     const CNI_CONF_DIR: &str = "/etc/cni/net.d";

//     fn init_test_net_fs() {
//         util::init_net_fs(
//             Path::new(CNI_CONF_DIR),
//             TEST_CNI_CONF_FILENAME,
//             TEST_NETWORK_NAME,
//             TEST_BRIDGE_NAME,
//             TEST_SUBNET,
//             CNI_DATA_DIR,
//         )
//         .unwrap()
//     }

//     #[test]
//     #[ignore]
//     fn test_cni_resource() {
//         dotenv::dotenv().unwrap();
//         env_logger::init_from_env(env_logger::Env::new().default_filter_or("trace"));
//         init_test_net_fs();
//         let netns = util::netns_from_cid_and_cns("123456", "cns");

//         let net_namespace = netns::create(&netns).unwrap();
//         defer::defer!({
//             let _ = netns::remove(&netns);
//         });
//         net_namespace.path()

//         let result = cni_add_bridge(&netns, TEST_NETWORK_NAME);
//         log::debug!("add CNI result: {:?}", result);
//         assert!(
//             result.is_ok_and(|output| output.status.success()),
//             "Failed to add CNI"
//         );

//         defer::defer!({
//             let result = cni_del_bridge(&netns, TEST_NETWORK_NAME);
//             log::debug!("del CNI result: {:?}", result);
//             assert!(
//                 result.is_ok_and(|output| output.status.success()),
//                 "Failed to delete CNI"
//             );
//         });
//     }
// }
