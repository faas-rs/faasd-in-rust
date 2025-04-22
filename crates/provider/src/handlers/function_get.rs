use crate::handlers::function_list::Function;
// use service::spec::{ Mount, Spec};
use actix_web::cookie::time::Duration;
use service::{containerd_manager::ContainerdManager, image_manager::ImageManager};
use std::{collections::HashMap, time::UNIX_EPOCH};
use thiserror::Error;

const ANNOTATION_LABEL_PREFIX: &str = "com.openfaas.annotations.";

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Runtime Config not found: {0}")]
    RuntimeConfigNotFound(String),
}

impl From<Box<dyn std::error::Error>> for FunctionError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        FunctionError::FunctionNotFound(error.to_string())
    }
}

pub async fn get_function(function_name: &str, namespace: &str) -> Result<Function, FunctionError> {
    let cid = function_name;
    let address = ContainerdManager::get_address(cid);

    let container = ContainerdManager::load_container(cid, namespace)
        .await
        .map_err(|e| FunctionError::FunctionNotFound(e.to_string()))?
        .unwrap_or_default();

    let container_name = container.id.to_string();
    let image = container.image.clone();
    let mut pid = 0;
    let mut replicas = 0;

    let all_labels = container.labels;
    let (labels, _) = build_labels_and_annotations(all_labels);

    let env = ImageManager::get_runtime_config(&image)
        .map_err(|e| FunctionError::RuntimeConfigNotFound(e.to_string()))?
        .env;
    let (env_vars, env_process) = read_env_from_process_env(env);
    // let secrets = read_secrets_from_mounts(&spec.mounts);
    // let memory_limit = read_memory_limit_from_spec(&spec);
    let timestamp = container.created_at.unwrap_or_default();
    let created_at = UNIX_EPOCH + Duration::new(timestamp.seconds, timestamp.nanos);

    let task = ContainerdManager::get_task(cid, namespace)
        .await
        .map_err(|e| FunctionError::FunctionNotFound(e.to_string()));
    match task {
        Ok(task) => {
            let status = task.status;
            if status == 2 || status == 3 {
                pid = task.pid;
                replicas = 1;
            }
        }
        Err(e) => {
            log::error!("Failed to get task: {}", e);
            replicas = 0;
        }
    }

    Ok(Function {
        name: container_name,
        namespace: namespace.to_string(),
        image,
        pid,
        replicas,
        address,
        labels,
        env_vars,
        env_process,
        created_at,
    })
}

fn build_labels_and_annotations(
    ctr_labels: HashMap<String, String>,
) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut labels = HashMap::new();
    let mut annotations = HashMap::new();

    for (k, v) in ctr_labels {
        if k.starts_with(ANNOTATION_LABEL_PREFIX) {
            annotations.insert(k.trim_start_matches(ANNOTATION_LABEL_PREFIX).to_string(), v);
        } else {
            labels.insert(k, v);
        }
    }

    (labels, annotations)
}

fn read_env_from_process_env(env: Vec<String>) -> (HashMap<String, String>, String) {
    let mut found_env = HashMap::new();
    let mut fprocess = String::new();

    for e in env {
        let kv: Vec<&str> = e.splitn(2, '=').collect();
        if kv.len() == 1 {
            continue;
        }
        if kv[0] == "PATH" {
            continue;
        }
        if kv[0] == "fprocess" {
            fprocess = kv[1].to_string();
            continue;
        }
        found_env.insert(kv[0].to_string(), kv[1].to_string());
    }

    (found_env, fprocess)
}

// fn read_secrets_from_mounts(mounts: &[Mount]) -> Vec<String> {
//     let mut secrets = Vec::new();
//     for mnt in mounts {
//         let parts: Vec<&str> = mnt.destination.split("/var/openfaas/secrets/").collect();
//         if parts.len() > 1 {
//             secrets.push(parts[1].to_string());
//         }
//     }
//     secrets
// }

// fn read_memory_limit_from_spec(spec: &Spec) -> i64 {
//     match &spec.linux {
//         linux => match &linux.resources {
//             resources => match &resources.memory {
//                 Some(memory) => memory.limit.unwrap_or(0),
//                 None => 0,
//             },
//             _ => 0,
//         },
//         _ => 0,
//     }
// }
