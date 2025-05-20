type Err = Box<dyn std::error::Error>;

use serde_json::Value;
use std::{fmt::Error, net::IpAddr, path::Path, sync::LazyLock};

use super::{command as cmd, netns, util};

static CNI_CONF_DIR: LazyLock<String> = LazyLock::new(|| {
    std::env::var("CNI_CONF_DIR").unwrap_or_else(|_| "/etc/cni/net.d".to_string())
});

const CNI_DATA_DIR: &str = "/var/run/cni";
const DEFAULT_CNI_CONF_FILENAME: &str = "10-faasrs.conflist";
const DEFAULT_NETWORK_NAME: &str = "faasrs-cni-bridge";
const DEFAULT_BRIDGE_NAME: &str = "faasrs0";
const DEFAULT_SUBNET: &str = "10.66.0.0/16";

pub fn init_cni_network() -> Result<(), Err> {
    util::init_net_fs(
        Path::new(CNI_CONF_DIR.as_str()),
        DEFAULT_CNI_CONF_FILENAME,
        DEFAULT_NETWORK_NAME,
        DEFAULT_BRIDGE_NAME,
        DEFAULT_SUBNET,
        CNI_DATA_DIR,
    )
}

//TODO: 创建网络和删除网络的错误处理
pub fn create_cni_network(cid: &str, ns: &str) -> Result<String, Err> {
    let netns = util::netns_from_cid_and_cns(cid, ns);
    let mut ip = String::new();

    netns::create(&netns)?;

    let output = cmd::cni_add_bridge(netns.as_str(), DEFAULT_NETWORK_NAME);

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Err(Box::new(Error));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let json: Value = match serde_json::from_str(&stdout) {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Failed to parse JSON: {}", e);
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
            log::error!("Failed to add CNI bridge: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(ip)
}

pub fn delete_cni_network(ns: &str, cid: &str) {
    let netns = util::netns_from_cid_and_cns(cid, ns);

    let _ = cmd::cni_del_bridge(&netns, DEFAULT_NETWORK_NAME);
    let _ = netns::remove(&netns);
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
