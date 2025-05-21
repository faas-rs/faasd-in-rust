use crate::consts;

pub mod cni_impl;
mod command;
mod util;

pub use cni_impl::init_cni_network;
use gateway::types::function::Query;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Endpoint {
    pub service: String,
    pub namespace: String,
}

impl Endpoint {
    pub fn new(service: &str, namespace: &str) -> Self {
        Self {
            service: service.to_string(),
            namespace: namespace.to_string(),
        }
    }
}

/// format `<namespace>-<service>` as netns name, also the identifier of each function
impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.namespace, self.service)
    }
}

impl From<Query> for Endpoint {
    fn from(query: Query) -> Self {
        Self {
            service: query.service,
            namespace: query
                .namespace
                .unwrap_or(consts::DEFAULT_FUNCTION_NAMESPACE.to_string()),
        }
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
