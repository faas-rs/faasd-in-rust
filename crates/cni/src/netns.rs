use netns_rs::{Error, NetNs};

pub(super) fn create(netns: &str) -> Result<NetNs, Error> {
    NetNs::new(netns)
}

pub(super) fn remove(netns: &str) -> Result<(), Error> {
    match NetNs::get(netns) {
        Ok(ns) => {
            ns.remove()?;
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to get netns {}: {}", netns, e);
            Err(e)
        }
    }
}

/// THESE TESTS SHOULD BE RUN WITH ROOT PRIVILEGES
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn test_create_and_remove() {
        let netns_name = "test_netns";
        create(netns_name).unwrap();
        assert!(remove(netns_name).is_ok());
    }
}
