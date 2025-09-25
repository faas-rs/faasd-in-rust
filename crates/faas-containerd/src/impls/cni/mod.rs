use crate::consts;

pub mod cni_impl;
mod command;
mod util;

pub use cni_impl::init_cni_network;
use gateway::types::function::Query;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Endpoint {
    pub function_name: String,
    pub namespace: String,
}

impl Endpoint {
    pub fn new(function_name: &str, namespace: &str) -> Self {
        Self {
            function_name: function_name.to_string(),
            namespace: namespace.to_string(),
        }
    }
}

/// format `<namespace>-<function_name>` as netns name, also the identifier of each function
impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.namespace, self.function_name)
    }
}

impl From<Query> for Endpoint {
    fn from(query: Query) -> Self {
        Self {
            function_name: query.function_name,
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
