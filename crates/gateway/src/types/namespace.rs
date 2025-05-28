use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Namespace {
    pub name: Option<String>,
    pub labels: HashMap<String, String>,
}
