use std::net::IpAddr;

use crate::impls::error::ContainerdError;

#[derive(Debug)]
pub struct CNIEndpoint {
    pub cid: String,
    pub ns: String,
    pub raw_ip: String,
    pub ips: Vec<IpAddr>,
}

impl CNIEndpoint {
    pub fn new(cid: &str, ns: &str) -> Result<Self, ContainerdError> {
        let raw_ip = cni::create_cni_network(cid, ns).map_err(|e| {
            log::error!("Failed to create CNI network: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;
        log::trace!("CNI network created with Raw IP: {:?}", raw_ip);

        let mut ips: Vec<IpAddr> = Vec::new();
        for ip in raw_ip.split('/') {
            if let Ok(parsed_ip) = ip.parse::<IpAddr>() {
                ips.push(parsed_ip);
            } else {
                log::error!("Failed to parse IP address: {}", ip);
                return Err(ContainerdError::CreateContainerError(format!(
                    "Failed to parse IP address: {}",
                    ip
                )));
            }
        }

        if ips.is_empty() {
            // TODO drop resources allocated
            log::error!("No valid IP address found in CNI network: {}", raw_ip);
            return Err(ContainerdError::CreateContainerError(format!(
                "No valid IP address found in CNI network: {}",
                raw_ip
            )));
        }

        log::info!("CNI network created with IP: {:?}", ips);
        Ok(Self {
            cid: cid.to_string(),
            ns: ns.to_string(),
            raw_ip,
            ips,
        })
    }

    fn delete(&mut self) {
        cni::delete_cni_network(&self.ns, &self.cid);
    }

    pub fn address(&self) -> IpAddr {
        self.ips[0]
    }
}

impl Drop for CNIEndpoint {
    fn drop(&mut self) {
        log::info!("Dropping CNIEndpoint");
        self.delete();
    }
}
