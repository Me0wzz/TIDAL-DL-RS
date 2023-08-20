use std::{fs, path::Path, process::exit, thread, time::Duration};

use crate::config::*;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Response,
};
use serde_json::{json, Value};

use crate::constants::{TIDAL_AUTH_LINK, TIDAL_BASE, USER_AGENT};

#[derive(Debug)]
pub struct TidalClient {
    pub device_code: DeviceCode,
    pub user_info: UserInfo,
    pub config: Config,
    pub have_userinfo: bool,
}
#[derive(Debug, Default)]
pub struct UserInfo {
    pub access_token: String,
    pub expires_in: String,
    pub country_code: String,
    pub user_id: String,
    pub refresh_token: String,
}
#[derive(Debug, Default)]
pub struct DeviceCode {
    pub device_code: String,
    pub expires_in: usize,
    pub interval: usize,
    pub user_code: String,
}

impl TidalClient {
    pub fn new(user_info: Option<(UserInfo, Config)>) -> TidalClient {
        match user_info {
            Some((user_info)) => TidalClient {
                device_code: DeviceCode::default(),
                user_info: user_info.0,
                have_userinfo: true,
                config: user_info.1,
            },
            None => TidalClient {
                device_code: DeviceCode::default(),
                user_info: UserInfo::default(),
                have_userinfo: false,
                config: Config::new(),
            },
        }
    }

    //pub async fn login() {}

    //id == client_id
    pub async fn get_session(&mut self, id: String) {
        if self.have_userinfo {
            println!("Session exists!\nSkip login..");
            println!("{:?}", self);
            return;
        }
        let mut header: HeaderMap = HeaderMap::new();
        header.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let payload = json!({
            "client_id": id.clone(),
            "scope": "r_usr+w_usr+w_sub",
        });
        let payload = serde_urlencoded::to_string(&payload).unwrap();
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap();
        let device_auth_url = format!("{}/device_authorization", TIDAL_AUTH_LINK);
        let response = self
            .api_post(client.clone(), device_auth_url, payload)
            .await;
        let val = response.json::<serde_json::Value>().await.unwrap();
        self.device_code = DeviceCode {
            device_code: remove_non_alphanumeric((*val.get("deviceCode").unwrap()).to_string()),
            expires_in: (*val.get("expiresIn").unwrap())
                .to_string()
                .parse::<usize>()
                .unwrap(),
            interval: (*val.get("interval").unwrap())
                .to_string()
                .parse::<usize>()
                .unwrap(),
            user_code: remove_non_alphanumeric((*val.get("userCode").unwrap()).to_string()),
        };
        let payload_d = json!({
            "client_id": "zU4XHVVkc2tDPo4t",
            "device_code": self.device_code.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
            "scope": "r_usr+w_usr+w_sub",
        });
        let payload_d2 = serde_urlencoded::to_string(&payload_d).unwrap();
        let mut elapsed = 0;
        println!(
            "connect via https://link.tidal.com/{}\nplease login before 300s",
            self.device_code.user_code
        );
        while elapsed < 300 {
            let a = self
                .check_auth_token(client.clone(), payload_d2.clone())
                .await;
            if a != 200 {
                thread::sleep(Duration::from_millis(1000));
                elapsed += 1;
                continue;
            } else {
                println!("Connected Tidal successfully");
                println!("{:?}", self);
                break;
            }
        }
        if elapsed >= 300 {
            println!("timeout!\nretry later");
            exit(0);
        }

        // println!("{:?}", response.status());
    }
    async fn api_post(&self, client: Client, url: String, data: String) -> Response {
        let response = client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await
            .unwrap();
        response
    }

    async fn check_auth_token(&mut self, client: Client, data: String) -> usize {
        let auth_token_url = format!("{}/token", TIDAL_AUTH_LINK);
        let response = self.api_post(client, auth_token_url, data).await;
        let tmp = response.json::<serde_json::Value>().await.unwrap();
        match tmp.get("status") {
            Some(status) => return status.to_string().parse::<usize>().unwrap(),
            None => {
                self.user_info.access_token =
                    remove_non_alphanumeric((tmp.get("access_token").unwrap()).to_string());
                self.user_info.country_code = remove_non_alphanumeric(
                    tmp.get("user")
                        .unwrap()
                        .get("countryCode")
                        .unwrap()
                        .to_string(),
                );
                self.user_info.expires_in =
                    remove_non_alphanumeric((tmp.get("expires_in").unwrap()).to_string());
                self.user_info.user_id = remove_non_alphanumeric(
                    (tmp.get("user").unwrap().get("userId").unwrap()).to_string(),
                );
                self.user_info.refresh_token =
                    remove_non_alphanumeric((tmp.get("refresh_token").unwrap()).to_string());
                return 200;
            }
        }
    }

    pub async fn login_session(&self) {
        let mut header: HeaderMap = HeaderMap::new();
        let token = format!("Bearer {}", self.user_info.access_token);
        header.insert(
            "authorization",
            HeaderValue::from_str(token.as_str()).unwrap(),
        );
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap();
        client
            .get(format!("{}/sessions", TIDAL_BASE))
            .headers(header)
            .send()
            .await
            .unwrap();
    }

    pub async fn save_token(&self) {
        let tdl_token_exist = Path::new(".tdlrs.json").exists();
        if !tdl_token_exist {
            fs::File::create(".tdlrs.json").expect("failed to create file");
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .append(false)
            .open(".tdlrs.json")
            .expect("failed to read file");
        // TODO: Implement enum to some T to write json properly
        let json = json!({
            "access_token": self.user_info.access_token,
            "expires_in": self.user_info.expires_in,
            "country_code": self.user_info.country_code,
            "user_id": self.user_info.user_id,
            "refresh_token": self.user_info.refresh_token,
            "download_path": self.config.download_path,
            "audio_quality": self.config.audio_quality,
            "save_cover": self.config.save_cover.to_string(),
            "exist_check": self.config.exist_check.to_string(),
        });

        serde_json::to_writer_pretty(&file, &json).unwrap();
    }
}
pub async fn get_token() -> Option<(UserInfo, Config)> {
    let exist = Path::new(".tdlrs.json").exists();
    if !exist {
        //fs::File::create(".tdlrs.json").expect("failed to create file");
        return None;
    }
    //let file = fs::File::open(".tdlrs.json").unwrap();
    let json: Value = serde_json::from_str(
        fs::read_to_string(".tdlrs.json")
            .expect("cannot open .tdlrs.json!")
            .as_str(),
    )
    .expect("cannot parse json");
    let user_info = UserInfo {
        access_token: json
            .get("access_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        expires_in: json
            .get("expires_in")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        country_code: json
            .get("country_code")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        user_id: json.get("user_id").unwrap().as_str().unwrap().to_string(),
        refresh_token: json
            .get("refresh_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
    };
    let config = Config {
        download_path: json
            .get("download_path")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        audio_quality: json
            .get("audio_quality")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
            .parse::<AudioQuality>()
            .unwrap(),
        save_cover: json
            .get("save_cover")
            .unwrap()
            .as_str()
            .unwrap()
            .trim()
            .parse::<bool>()
            .unwrap(),
        exist_check: json
            .get("exist_check")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
            .trim()
            .parse::<bool>()
            .unwrap(),
    };
    Some((user_info, config))
}

pub fn remove_non_alphanumeric(s: String) -> String {
    s.replace("\"", "")
        .replace("\\", "")
        .replace("[", "")
        .replace("]", "")
}
