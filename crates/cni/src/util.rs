use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

static mut CNI_CONFIG_FILE: Option<CniConfFile> = None;

/// Generate "cns-cid"
#[inline(always)]
pub fn netns_from_cid_and_cns(cid: &str, cns: &str) -> String {
    format!("{}-{}", cns, cid)
}

pub fn init_net_fs(
    conf_dir: &Path,
    conf_filename: &str,
    net_name: &str,
    bridge: &str,
    subnet: &str,
    data_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conf_file = CniConfFile::new(conf_dir, conf_filename, net_name, bridge, subnet, data_dir)?;
    unsafe {
        CNI_CONFIG_FILE = Some(conf_file);
    }
    Ok(())
}

fn cni_conf(name: &str, bridge: &str, subnet: &str, data_dir: &str) -> String {
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
        name, bridge, subnet, data_dir
    )
}

struct CniConfFile {
    conf_dir: PathBuf,
    conf_filename: String,
}

impl CniConfFile {
    fn new(
        conf_dir: &Path,
        conf_filename: &str,
        net_name: &str,
        bridge: &str,
        subnet: &str,
        data_dir: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if !conf_dir.exists() {
            std::fs::create_dir_all(conf_dir)?;
        }
        if !conf_dir.is_dir() {
            log::error!("CNI_CONF_DIR is not a directory");
            panic!("CNI_CONF_DIR is not a directory");
        }
        let net_config = conf_dir.join(conf_filename);
        File::create(&net_config)?
            .write_all(cni_conf(net_name, bridge, subnet, data_dir).as_bytes())?;
        Ok(Self {
            conf_dir: conf_dir.to_path_buf(),
            conf_filename: conf_filename.to_string(),
        })
    }
}

impl Drop for CniConfFile {
    fn drop(&mut self) {
        let net_config = self.conf_dir.join(&self.conf_filename);
        if net_config.exists() {
            std::fs::remove_file(&net_config).unwrap();
        }
    }
}
