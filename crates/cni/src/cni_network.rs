use crate::Err;
use std::{fs::{self, File}, io::Write, path::Path};
const CNI_BIN_DIR: &str = "/opt/cni/bin";
const CNI_CONF_DIR: &str = "/etc/cni/net.d";
const NET_NS_PATH_FMT: &str = "/proc/{}/ns/net";
const CNI_DATA_DIR: &str = "/var/run/cni";
const DEFAULT_CNI_CONF_FILENAME: &str = "10-openfaas.conflist";
const DEFAULT_NETWORK_NAME: &str = "openfaas-cni-bridge";
const DEFAULT_BRIDGE_NAME: &str = "openfaas0";
const DEFAULT_SUBNET: &str = "10.62.0.0/16";
const DEFAULT_IF_PREFIX: &str = "eth";

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
    if !dir_exists(CNI_CONF_DIR) {
        fs::create_dir_all(CNI_CONF_DIR)?;
    }
    let net_config = Path::new(CNI_CONF_DIR).join(DEFAULT_CNI_CONF_FILENAME);
    let mut file = File::create(&net_config)?;
    file.write_all(default_cni_conf().as_bytes())?;


    Ok(())
}

pub fn create_network()

fn dir_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}
