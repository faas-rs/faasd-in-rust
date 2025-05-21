#![feature(ip_from)]
#![feature(slice_as_array)]
pub mod consts;
pub mod impls;
pub mod provider;
pub mod systemd;

pub use impls::init_backend;
