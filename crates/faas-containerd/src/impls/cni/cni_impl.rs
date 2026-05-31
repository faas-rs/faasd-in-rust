type Err = Box<dyn std::error::Error>;

use derive_more::{Display, Error};
use netns_rs::NetNs;
use serde_json::Value;
use std::{net::IpAddr, path::Path, sync::LazyLock};

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

// ── Bracket: acquire CNI bridge, run f, auto-cleanup on error ──────────
///
/// After `cni_add_bridge` succeeds, any resource acquired (bridge + netns)
/// must be released on error. On success, the caller keeps ownership and
/// cleanup is suppressed.
pub fn create_cni_network(
    cx: &asupersync::Cx,
    endpoint: &Endpoint,
) -> Result<(cidr::IpInet, NetNs), NetworkError> {
    cx.checkpoint().ok();
    cx.trace("cni:creating");

    let net_ns = NetNs::new(endpoint.to_string()).map_err(|e| NetworkError {
        msg: format!("Failed to create netns: {}", e),
    })?;

    let output = cmd::cni_add_bridge(net_ns.path(), DEFAULT_NETWORK_NAME);
    let output = match output {
        Ok(o) => o,
        Err(e) => {
            net_ns.remove().ok();
            return Err(NetworkError {
                msg: format!("Failed to add CNI bridge: {e}"),
            });
        }
    };

    if !output.status.success() {
        net_ns.remove().ok();
        return Err(NetworkError {
            msg: format!(
                "Failed to add CNI bridge: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }
    // ── Bridge acquired; every error path cleans up bridge + netns ──
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut json: Value = match serde_json::from_str(&stdout) {
        Ok(j) => j,
        Err(e) => {
            let _ = cmd::cni_del_bridge(net_ns.path(), DEFAULT_NETWORK_NAME);
            net_ns.remove().ok();
            return Err(NetworkError {
                msg: format!("Failed to parse CNI JSON: {e}"),
            });
        }
    };

    log::trace!("CNI add bridge output: {:?}", json);
    let ips = json["ips"].take();
    let ip_list = match ips {
        Value::Array(ref arr) if !arr.is_empty() => {
            let mut list = Vec::with_capacity(arr.len());
            for ip in arr {
                if let Value::String(ref ip_str) = ip["address"] {
                    list.push(ip_str.parse::<cidr::IpInet>().map_err(|e| NetworkError {
                        msg: format!("Failed to parse IP address: {}", e),
                    })?);
                }
            }
            list
        }
        _ => {
            return Err(NetworkError {
                msg: "No IP address found in CNI output".to_string(),
            });
        }
    };

    if ip_list.len() > 1 {
        log::warn!("Multiple IP addresses in CNI output: {:?}", ip_list);
    }
    log::trace!("CNI network created with IP: {:?}", ip_list[0]);
    Ok((ip_list[0], net_ns))
}

pub fn delete_cni_network(cx: &asupersync::Cx, endpoint: &Endpoint) -> Result<(), NetworkError> {
    cx.checkpoint().ok();
    cx.trace("cni:deleting");

    match NetNs::get(endpoint.to_string()) {
        Ok(ns) => {
            let e1 = cmd::cni_del_bridge(ns.path(), DEFAULT_NETWORK_NAME);
            let e2 = ns.remove();
            if e1.is_err() || e2.is_err() {
                let msg =
                    format!("NetNS exists but cleanup failed: cni_bridge={e1:?}, netns={e2:?}");
                log::error!("{msg}");
                return Err(NetworkError { msg });
            }
            cx.trace("cni:deleted");
            Ok(())
        }
        Err(e) => {
            let msg = format!("Failed to get netns {endpoint}: {e}");
            log::warn!("{msg}");
            Err(NetworkError { msg })
        }
    }
}

#[inline]
pub fn check_network_exists(addr: IpAddr) -> bool {
    use std::process::Command;
    Command::new("ip")
        .args(["addr", "show", &addr.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[allow(unused)]
fn cni_gateway() -> Result<String, Err> {
    let content = std::fs::read_to_string(format!(
        "{}/{}",
        CNI_CONF_DIR.as_str(),
        DEFAULT_CNI_CONF_FILENAME
    ))?;
    let json: Value = serde_json::from_str(&content)?;
    json["plugins"][0]["ipam"]["routes"][0]["gw"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing gateway".into())
}
