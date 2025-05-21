type Err = Box<dyn std::error::Error>;

use derive_more::{Display, Error};
use netns_rs::NetNs;
use scopeguard::{ScopeGuard, guard};
use serde_json::Value;
use std::{fmt::Error, net::IpAddr, path::Path, sync::LazyLock};

use super::{Endpoint, command as cmd, util};

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

#[derive(Debug, Display, Error)]
pub struct NetworkError {
    pub msg: String,
}

//TODO: 创建网络和删除网络的错误处理
pub fn create_cni_network(endpoint: &Endpoint) -> Result<(cidr::IpInet, NetNs), NetworkError> {
    let net_ns = guard(
        NetNs::new(endpoint.to_string()).map_err(|e| NetworkError {
            msg: format!("Failed to create netns: {}", e),
        })?,
        |ns| ns.remove().unwrap(),
    );

    let output = cmd::cni_add_bridge(net_ns.path(), DEFAULT_NETWORK_NAME);

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Err(NetworkError {
                    msg: format!(
                        "Failed to add CNI bridge: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut json: Value = match serde_json::from_str(&stdout) {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Failed to parse JSON: {}", e);
                    return Err(NetworkError {
                        msg: format!("Failed to parse JSON: {}", e),
                    });
                }
            };
            log::trace!("CNI add bridge output: {:?}", json);
            if let serde_json::Value::Array(ips) = json["ips"].take() {
                let mut ip_list = Vec::new();
                for mut ip in ips {
                    if let serde_json::Value::String(ip_str) = ip["address"].take() {
                        let ipcidr = ip_str.parse::<cidr::IpInet>().map_err(|e| {
                            log::error!("Failed to parse IP address: {}", e);
                            NetworkError { msg: e.to_string() }
                        })?;
                        ip_list.push(ipcidr);
                    }
                }
                if ip_list.is_empty() {
                    return Err(NetworkError {
                        msg: "No IP address found in CNI output".to_string(),
                    });
                }
                if ip_list.len() > 1 {
                    log::warn!("Multiple IP addresses found in CNI output: {:?}", ip_list);
                }
                log::trace!("CNI network created with IP: {:?}", ip_list[0]);
                Ok((ip_list[0], ScopeGuard::into_inner(net_ns)))
            } else {
                log::error!("Invalid JSON format: {:?}", json);
                Err(NetworkError {
                    msg: "Invalid JSON format".to_string(),
                })
            }
        }
        Err(e) => {
            log::error!("Failed to add CNI bridge: {}", e);
            Err(NetworkError {
                msg: format!("Failed to add CNI bridge: {}", e),
            })
        }
    }
}

pub fn delete_cni_network(endpoint: Endpoint) -> Result<(), NetworkError> {
    match NetNs::get(endpoint.to_string()) {
        Ok(ns) => {
            let e1 = cmd::cni_del_bridge(ns.path(), DEFAULT_NETWORK_NAME);
            let e2 = ns.remove();
            if e1.is_err() || e2.is_err() {
                let err = format!(
                    "NetNS exists, but failed to delete CNI network, cni bridge: {:?}, netns: {:?}",
                    e1, e2
                );
                log::error!("{}", err);
                return Err(NetworkError { msg: err });
            }
            Ok(())
        }
        Err(e) => {
            let msg = format!("Failed to get netns {}: {}", endpoint, e);
            log::warn!("{}", msg);
            Err(NetworkError { msg })
        }
    }
}

#[inline]
pub fn check_network_exists(addr: IpAddr) -> bool {
    util::CNI_CONFIG_FILE
        .get()
        .unwrap()
        .data_dir
        .join(addr.to_string())
        .exists()
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
