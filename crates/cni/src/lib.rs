type Err = Box<dyn std::error::Error>;

use lazy_static::lazy_static;
use netns_rs::NetNs;
use serde_json::Value;
use std::{
    fmt::Error,
    fs::{self, File},
    io::Write,
    net::IpAddr,
    path::Path,
};

lazy_static! {
    static ref CNI_BIN_DIR: String =
        std::env::var("CNI_BIN_DIR").expect("Environment variable CNI_BIN_DIR is not set");
    static ref CNI_CONF_DIR: String =
        std::env::var("CNI_CONF_DIR").expect("Environment variable CNI_CONF_DIR is not set");
    static ref CNI_TOOL: String =
        std::env::var("CNI_TOOL").expect("Environment variable CNI_TOOL is not set");
}

// const NET_NS_PATH_FMT: &str = "/proc/{}/ns/net";
const CNI_DATA_DIR: &str = "/var/run/cni";
const DEFAULT_CNI_CONF_FILENAME: &str = "10-faasrs.conflist";
const DEFAULT_NETWORK_NAME: &str = "faasrs-cni-bridge";
const DEFAULT_BRIDGE_NAME: &str = "faasrs0";
const DEFAULT_SUBNET: &str = "10.66.0.0/16";
// const DEFAULT_IF_PREFIX: &str = "eth";

fn default_cni_conf() -> String {
    format!(
        r#"
{{
    "cniVersion": "0.4.0",
    "name": "{}",
    "plugins": [
      {{
        "type": "bridge",
        "bridge": "{}",
        "isGateway": true,
        "ipMasq": true,
        "ipam": {{
            "type": "host-local",
            "subnet": "{}",
            "dataDir": "{}",
            "routes": [
                {{ "dst": "0.0.0.0/0" }}
            ]
        }}
      }},
      {{
        "type": "firewall"
      }}
    ]
}}
"#,
        DEFAULT_NETWORK_NAME, DEFAULT_BRIDGE_NAME, DEFAULT_SUBNET, CNI_DATA_DIR
    )
}

pub fn init_net_work() -> Result<(), Err> {
    let cni_conf_dir = CNI_CONF_DIR.as_str();
    if !dir_exists(Path::new(cni_conf_dir)) {
        fs::create_dir_all(cni_conf_dir)?;
    }
    let net_config = Path::new(cni_conf_dir).join(DEFAULT_CNI_CONF_FILENAME);
    let mut file = File::create(&net_config)?;
    file.write_all(default_cni_conf().as_bytes())?;

    Ok(())
}

fn get_netns(ns: &str, cid: &str) -> String {
    format!("{}-{}", ns, cid)
}

fn get_path(netns: &str) -> String {
    format!("/var/run/netns/{}", netns)
}

//TODO: 创建网络和删除网络的错误处理
pub fn create_cni_network(cid: String, ns: String) -> Result<(String, String), Err> {
    // let netid = format!("{}-{}", cid, pid);
    let netns = get_netns(ns.as_str(), cid.as_str());
    let path = get_path(netns.as_str());
    let mut ip = String::new();

    create_netns(&netns);

    let bin = CNI_BIN_DIR.as_str();
    let cnitool = CNI_TOOL.as_str();
    let output = std::process::Command::new(cnitool)
        .arg("add")
        .arg("faasrs-cni-bridge")
        .arg(&path)
        .env("CNI_PATH", bin)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Err(Box::new(Error));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let json: Value = match serde_json::from_str(&stdout) {
                Ok(json) => json,
                Err(e) => {
                    return Err(Box::new(e));
                }
            };
            if let Some(ips) = json.get("ips").and_then(|ips| ips.as_array()) {
                if let Some(first_ip) = ips
                    .first()
                    .and_then(|ip| ip.get("address"))
                    .and_then(|addr| addr.as_str())
                {
                    ip = first_ip.to_string();
                }
            }
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    }

    Ok((ip, path))
}

pub fn delete_cni_network(ns: &str, cid: &str) {
    let netns = get_netns(ns, cid);
    let path = get_path(&netns);
    let bin = CNI_BIN_DIR.as_str();
    let cnitool = CNI_TOOL.as_str();

    let _output_del = std::process::Command::new(cnitool)
        .arg("del")
        .arg("faasrs-cni-bridge")
        .arg(&path)
        .env("CNI_PATH", bin)
        .output();
    delete_netns(&netns);
}

fn create_netns(namespace_name: &str) {
    match NetNs::new(namespace_name) {
        Ok(ns) => {
            log::info!("Created netns: {}", ns);
        }
        Err(e) => {
            log::error!("Error creating netns: {}", e);
        }
    }
}

fn delete_netns(namespace_name: &str) {
    match NetNs::get(namespace_name) {
        Ok(ns) => {
            ns.remove()
                .map_err(|e| log::error!("Error deleting netns: {}", e))
                .unwrap();
            log::info!("Deleted netns: {}", namespace_name);
        }
        Err(e) => {
            log::error!("Error getting netns: {}, NotFound", e);
        }
    }
}

fn dir_exists(dirname: &Path) -> bool {
    path_exists(dirname).is_some_and(|info| info.is_dir())
}

fn path_exists(path: &Path) -> Option<fs::Metadata> {
    fs::metadata(path).ok()
}

#[allow(unused)]
fn cni_gateway() -> Result<String, Err> {
    let ip: IpAddr = DEFAULT_SUBNET.parse().unwrap();
    if let IpAddr::V4(ip) = ip {
        let octets = &mut ip.octets();
        octets[3] = 1;
        return Ok(ip.to_string());
    }
    Err(Box::new(Error))
}

#[allow(unused)]
fn dir_empty(dirname: &Path) -> bool {
    if !dir_exists(dirname) {
        return false;
    }
    match fs::read_dir(dirname) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => false,
    }
}
