use actix_web::{Error, HttpMessage, HttpResponse, dev::ServiceRequest};
use std::collections::HashMap;

//写到使用actix-web-httpauth作为中间件，还没有解决read_basic_auth函数的实现，返回值和之前在bootstrap的调用不一样

pub struct BasicAuthCredentials {
    user: String,
    password: String,
}

impl BasicAuthCredentials {
    pub fn new(username: &str, password: &str) -> Self {
        BasicAuthCredentials {
            user: username.to_string(),
            password: password.to_string(),
        }
    }
}

pub struct ReadBasicAuthFromDisk {
    secret_mount_path: String,
    user_filename: String,
    password_filename: String,
}

impl ReadBasicAuthFromDisk {
    pub fn new(secret_mount_path: &str, user_filename: &str, password_filename: &str) -> Self {
        ReadBasicAuthFromDisk {
            secret_mount_path: secret_mount_path.to_string(),
            user_filename: user_filename.to_string(),
            password_filename: password_filename.to_string(),
        }
    }
    //TODO:这里应该加密？
    pub async fn read_basic_auth(&self) -> HashMap<String, String> {
        let mut user_map = HashMap::new();
        let user_file =
            std::fs::read_to_string(format!("{}/{}", self.secret_mount_path, self.user_filename))
                .unwrap();
        let password_file = std::fs::read_to_string(format!(
            "{}/{}",
            self.secret_mount_path, self.password_filename
        ))
        .unwrap();
        let user_vec: Vec<&str> = user_file.split("\n").collect();
        let password_vec: Vec<&str> = password_file.split("\n").collect();
        for i in 0..user_vec.len() {
            user_map.insert(user_vec[i].to_string(), password_vec[i].to_string());
        }
        user_map
    }

    pub async fn basic_auth_validator(&self, req: ServiceRequest) -> Result<ServiceRequest, Error> {
        let auth_header = req.headers().get("Authorization");
        if let Some(auth_header) = auth_header {
            //TODO:to_str()转化失败的处理，或者在之前限制用户输入非法字符
            let auth_header = auth_header.to_str().unwrap();
            let auth_header = auth_header.split(" ").collect::<Vec<&str>>();
            if auth_header.len() != 2 {
                return Err(actix_web::error::ErrorUnauthorized(
                    "Invalid Authorization Header",
                ));
            }
            let auth_header = auth_header[1];
            let auth_header = base64::decode(auth_header).unwrap();
            let auth_header = String::from_utf8(auth_header).unwrap();
            let auth_header = auth_header.split(":").collect::<Vec<&str>>();
            if auth_header.len() != 2 {
                return Err(actix_web::error::ErrorUnauthorized(
                    "Invalid Authorization Header",
                ));
            }
            let username = auth_header[0];
            let password = auth_header[1];
            let user_map = self.read_basic_auth().await;
            if let Some(user) = user_map.get(username) {
                if user == password {
                    return Ok(req);
                }
            }
        }
        Err(actix_web::error::ErrorUnauthorized(
            "Invalid Username or Password",
        ))
    }
}

async fn index() -> HttpResponse {
    HttpResponse::Ok().body("欢迎访问受保护的资源！")
}
