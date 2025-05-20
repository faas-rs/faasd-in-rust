use std::net::IpAddr;

use crate::impls::error::ContainerdError;

mod cni_impl;
mod command;
mod netns;
mod util;

pub use cni_impl::init_cni_network;

#[derive(Debug)]
pub struct CNIEndpoint {
    pub cid: String,
    pub ns: String,
    pub ipcidr: cidr::IpInet,
}

impl CNIEndpoint {
    pub fn new(cid: &str, ns: &str) -> Result<Self, ContainerdError> {
        let raw_ip = cni_impl::create_cni_network(cid, ns).map_err(|e| {
            log::error!("Failed to create CNI network: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;
        log::trace!("CNI network created with Raw IP: {:?}", raw_ip);

        let ipcidr = raw_ip.parse::<cidr::IpInet>().map_err(|e| {
            log::error!("Failed to parse IP address: {}", e);
            ContainerdError::CreateContainerError(e.to_string())
        })?;

        log::info!("CNI network created with IP: {:?}", ipcidr);
        Ok(Self {
            cid: cid.to_string(),
            ns: ns.to_string(),
            ipcidr,
        })
    }

    fn delete(&mut self) {
        cni_impl::delete_cni_network(&self.ns, &self.cid);
    }

    pub fn address(&self) -> IpAddr {
        self.ipcidr.address()
    }
}

impl Drop for CNIEndpoint {
    fn drop(&mut self) {
        log::info!("Dropping CNIEndpoint");
        self.delete();
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ip_parsing() {
        let raw_ip = "10.42.0.48/16";
        let ipcidr = raw_ip.parse::<cidr::IpInet>().unwrap();
        assert_eq!(
            ipcidr.address(),
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 42, 0, 48))
        );
    }
}
