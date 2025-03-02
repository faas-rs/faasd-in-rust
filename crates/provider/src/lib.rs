pub mod config;
pub mod handler;
pub mod types;
pub mod httputils;
pub mod proxy;
pub mod auth;
pub mod logs;
pub mod metrics;
pub mod consts;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
