extern crate anyhow;
extern crate base64;
extern crate crypto_hash;
extern crate hex;
extern crate hmac;
extern crate rand;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate urlencoding;

use ::hmac::{Hmac, Mac};
use anyhow::{anyhow, Context, Result};
use crypto_hash::{hex_digest, Algorithm};
use hmac::NewMac;
use rand::{thread_rng, Rng};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use sha2::{Digest, Sha256};
use std::{
    iter,
    time::{SystemTime, UNIX_EPOCH},
    vec,
};
use uuid::Uuid;

fn parse_response_json(str: &str) -> serde_json::Result<Value> {
    let str = if str.starts_with("&&&START&&&") {
        &str[11..]
    } else {
        str
    };

    serde_json::from_str(str)
}

fn serde_value_to_string(value: &Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => serde_json::to_string(value).unwrap(),
    }
}

fn object_to_query_string(obj: &Value) -> String {
    let mut obj_as_query_string = String::new();
    if let Some(obj) = obj.as_object() {
        for (k, v) in obj {
            obj_as_query_string.push_str(&format!(
                r"&{}={}",
                k,
                urlencoding::encode(serde_value_to_string(v).as_str())
            ));
        }
    };
    obj_as_query_string.replacen('&', "", 1)
}

#[derive(serde::Serialize)]
pub struct UrlsConfig {
    cn: String,
    de: String,
    ru: String,
    sg: String,
    tw: String,
    us: String,
    login_step1: String,
    login_step2: String,
}

impl UrlsConfig {
    fn to_json(&self) -> Value {
        json!(self)
    }
}

pub struct MiCloudProtocol {
    urls: UrlsConfig,
    username: Option<String>,
    password_md5: Option<String>,
    ssecurity: Option<String>,
    user_id: Option<String>,
    country: String,
    service_token: Option<String>,
    user_agent: String,
    client_id: String,
    locale: &'static str,
}

#[derive(Serialize, Deserialize)]
struct MiCloudOkResponse<T = Value> {
    result: T,
}

#[derive(Serialize, Deserialize)]
struct MiCloudErrorMessageResponse {
    message: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct MiCloudErrorResponse {
    error: MiCloudErrorMessageResponse,
}

#[derive(Serialize, Deserialize)]
struct DeviceListResponse {
    list: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    adminFlag: Number,
    bssid: String,
    desc: String,
    did: String,
    extra: Value,
    family_id: Number,
    isOnline: bool,
    latitude: String,
    localip: String,
    longitude: String,
    mac: String,
    model: String,
    name: String,
    p2p_id: String,
    parent_id: String,
    parent_model: String,
    password: String,
    pd_id: Number,
    permitLevel: Number,
    pid: String,
    reset_flag: Number,
    rssi: Number,
    shareFlag: Number,
    show_mode: Number,
    ssid: String,
    token: String,
    uid: Number,
}

impl MiCloudProtocol {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let agent_id: String =
            iter::repeat_with(|| b"ABCDEF"[rng.gen_range(0..b"ABCDEF".len())] as char)
                .take(13)
                .collect();

        let uuidv4 = Uuid::new_v4();
        // Another known type of client_id (deviceId) is "wb_{uuidv4}", which likely stands for "web browser"
        let client_id: String = format!("android_{uuidv4}").to_string();

        let xiaomi_base_url = "api.io.mi.com/app";
        MiCloudProtocol {
            urls: UrlsConfig {
                cn: format!("https://{}", xiaomi_base_url),
                de: format!("https://de.{}", xiaomi_base_url),
                ru: format!("https://ru.{}", xiaomi_base_url),
                sg: format!("https://sg.{}", xiaomi_base_url),
                tw: format!("https://tw.{}", xiaomi_base_url),
                us: format!("https://us.{}", xiaomi_base_url),
                login_step1: "https://account.xiaomi.com/pass/serviceLogin".to_string(),
                login_step2: "https://account.xiaomi.com/pass/serviceLoginAuth2".to_string(),
            },
            username: None,
            password_md5: None,
            ssecurity: None,
            country: "cn".to_string(),
            user_id: None,
            service_token: None,
            user_agent: format!(
                "Android-7.1.1-1.0.0-ONEPLUS A3010-136-{} APP/xiaomi.smarthome APPV/62830",
                agent_id.clone()
            ),
            client_id,
            locale: "en",
        }
    }

    pub fn get_available_countries(&self) -> Vec<Vec<&'static str>> {
        vec![
            vec!["cn", "China"],
            vec!["ru", "Russia"],
            vec!["us", "USA"],
            vec!["i2", "India"],
            vec!["tw", "Taiwan"],
            vec!["sg", "Singapore"],
            vec!["de", "Germany"],
        ] // https://www.openhab.org/addons/bindings/miio/#country-servers
    }

    /// Authenticates a user with Mi Cloud.
    ///
    /// # Arguments
    ///
    /// * `username` - The username (email) for logging into Mi Cloud.
    /// * `password` - The password for the user.
    ///
    /// # Examples
    ///
    /// ```
    /// use mi_cloud_protocol::MiCloudProtocol;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut mi_cloud = MiCloudProtocol::new();
    ///     mi_cloud.login("my_username", "my_password").await.unwrap();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` if authentication fails.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let client = Client::new();
        let password_md5 = hex_digest(Algorithm::MD5, password.as_bytes()).to_uppercase();
        let (sign, _) = self.login_step1(&client, username).await?;
        let (ssecurity, user_id, location) = self
            .login_step2(&client, username, password_md5.as_str(), &sign)
            .await?;
        let service_token = self.login_step3(&client, location).await?;

        self.username = Some(username.to_string());
        self.password_md5 = Some(password_md5);
        self.ssecurity = Some(ssecurity);
        self.user_id = Some(user_id.to_string());
        self.service_token = Some(service_token);

        Ok(())
    }

    pub fn is_country_supported(&self, country: &str) -> bool {
        self.get_available_countries()
            .iter()
            .any(|x| x[0] == country)
    }

    pub fn set_country(&mut self, country: &str) {
        if self.is_country_supported(country) {
            self.country = country.to_string();
        }
    }

    pub async fn get_devices<'a>(
        &self,
        device_ids: Option<&'a [&'a str]>,
        country: Option<&str>,
    ) -> Result<Vec<Device>> {
        let req = if let Some(ids) = device_ids {
            json!({ "dids": ids })
        } else {
            json!({ "getVirtualModel": false, "getHuamiDevices": 0 })
        };

        let country = country.unwrap_or(self.country.as_str());
        let res = self
            .request("/home/device_list", req, country)
            .await
            .map_err(|e| anyhow!(e))?;

        if !res["result"].is_null() {
            let parsed_res: MiCloudOkResponse<DeviceListResponse> =
                serde_json::from_value(res.clone())?;
            let devices = parsed_res.result.list;
            Ok(devices)
        } else {
            let parsed_err: MiCloudErrorResponse = serde_json::from_value(res.clone())?;
            let message = parsed_err
                .error
                .message
                .unwrap_or("Get devices failed".to_string());
            Err(anyhow!(message))
        }
    }

    pub async fn get_device<'a>(
        &self,
        device_id: &str,
        country: Option<&str>,
    ) -> Result<Vec<Device>> {
        let device_ids = Some(vec![device_id]);
        self.get_devices(device_ids.as_deref(), country).await
    }

    pub async fn call_device<'a>(
        &self,
        device_id: &str,
        method: &str,
        params: Option<Value>,
        country: Option<&str>,
    ) -> Result<Value> {
        let req = json!({ "method": method, "params": params });

        let country = country.unwrap_or(self.country.as_str());
        let fallback_msg = format!("Miio call for device {} failed", device_id);
        let res = self
            .request(&format!(r"/home/rpc/{}", device_id), req, country)
            .await
            .with_context(|| fallback_msg.to_string())?;

        if !res["result"].is_null() {
            Ok(res["result"].clone())
        } else {
            let parsed_err: MiCloudErrorResponse = serde_json::from_value(res.clone())?;
            let message = parsed_err.error.message.unwrap_or(fallback_msg.to_string());
            Err(anyhow!(message))
        }
    }

    pub fn _override_urls(&mut self, urls: UrlsConfig) {
        self.urls = urls;
    }

    async fn login_step1(&self, client: &Client, username: &str) -> Result<(String, String)> {
        let url = self.urls.login_step1.clone();
        let mut query: Vec<(&'static str, String)> = vec![
            ("sid", "xiaomiio".to_string()),
            ("_json", "true".to_string()),
            ("_locale", "en_US".to_string()),
        ];

        let res = client
            .get(url)
            .header(header::HOST, "account.xiaomi.com")
            .header(
                header::COOKIE,
                format!("userId={}; deviceId={}", username, self.client_id),
            )
            .header(header::USER_AGENT, &self.user_agent)
            .query(&query)
            .send()
            .await?;

        let status = res.status();
        let content = res.text().await?;

        if !status.is_success() {
            return Err(anyhow!(format!(
                "Login step 1 failed: Response status {}",
                status
            ),));
        }

        let data = parse_response_json(&content)?;
        let sign = match data["_sign"].as_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(anyhow!("Login step 1 failed: No '_sign' in response"));
            }
        };

        Ok((sign, content))
    }

    async fn login_step2(
        &self,
        client: &Client,
        username: &str,
        password_md5: &str,
        sign: &str,
    ) -> Result<(String, i64, String)> {
        let form_data = vec![
            ("hash", password_md5.to_string().clone()),
            ("_json", "true".to_string()),
            ("_locale", "en_US".to_string()),
            ("sid", "xiaomiio".to_string()),
            ("callback", "https://sts.api.io.mi.com/sts".to_string()),
            (
                "qs",
                "%3Fsid%3Dxiaomiio%26_json%3Dtrue%26_locale%3Den_US".to_string(),
            ),
            ("_sign", sign.to_string()),
            ("user", username.to_string()),
            ("captCode", "".to_string()),
        ];

        let url = self.urls.login_step2.clone();
        let res = client
            .post(url)
            .form(&form_data)
            .header(header::HOST, "account.xiaomi.com")
            .header(header::COOKIE, format!("deviceId={}", self.client_id))
            .header(header::USER_AGENT, &self.user_agent)
            .send()
            .await?;

        let status = res.status();
        let content = res.text().await.unwrap_or("".to_string());

        if !status.is_success() {
            return Err(anyhow!(format!(
                "Login step 2 failed: Response status {}",
                status
            ),));
        }

        let data = parse_response_json(&content)?;

        if let Some(_) = data["captchaUrl"].as_str() {
            return Err(anyhow!("Login step 2 failed: Captcha"));
        }
        let ssecurity = match data["ssecurity"].as_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(anyhow!("Login step 2 failed: No 'ssecurity' in response"));
            }
        };
        let user_id = match data["userId"].as_i64() {
            Some(i) => i,
            None => {
                return Err(anyhow!("Login step 2 failed: No 'userId' in response"));
            }
        };
        let location = match data["location"].as_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(anyhow!("Login step 2 failed: No 'location' in response"));
            }
        };

        Ok((ssecurity, user_id, location))
    }

    async fn login_step3(&self, client: &Client, url: String) -> Result<String> {
        match client.get(url).send().await {
            Ok(response) => {
                let cookies_str = response.headers().values().collect::<Vec<_>>();
                for cookie_str in cookies_str {
                    for part in cookie_str.to_str().unwrap().split(';') {
                        if let Some(path) = part.trim().split_once('=') {
                            if path.0.trim() == "serviceToken" {
                                return Ok(path.1.trim().to_string());
                            }
                        }
                    }
                }
                Err(anyhow!(
                    "Login step 3 failed: No 'serviceToken' in response"
                ))
            }
            Err(e) => Err(anyhow!(e).context("Login step 3 failed")),
        }
    }

    async fn request(
        &self,
        path: &str,
        data: serde_json::Value,
        country: &str,
    ) -> Result<serde_json::Value> {
        let client = Client::new();

        if self.service_token.is_none() {
            return Err(anyhow!("Request error: Not logged in"));
        }

        if !self.is_country_supported(country) {
            return Err(anyhow!(
                "Request error: Server Location {} is not supported",
                country
            ));
        }

        let params = json!({"data": data});
        let url = format!("{}{}", self.get_api_url(country), path);
        let nonce = self.generate_nonce();
        let signed_nonce = self.signed_nonce(self.ssecurity.as_ref().unwrap(), &nonce);
        let signature = self.generate_signature(path, &signed_nonce, &nonce, &params);
        let body = json!({
            "_nonce": nonce,
            "data": data,
            "signature": signature
        });

        let body_as_query_string = object_to_query_string(&body);

        let res = client
            .post(&url)
            .header(header::USER_AGENT, self.user_agent.to_string())
            .header("x-xiaomi-protocal-flag-cli", "PROTOCAL-HTTP2")
            .header("mishop-client-id", "180100041079")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::COOKIE, self.get_cookie())
            .body(body_as_query_string)
            .send()
            .await
            .with_context(|| "Failed to send request")?;

        if !res.status().is_success() {
            return Err(anyhow!(
                "Request error: Status {}, {:#?}",
                res.status(),
                res
            ));
        }

        res.json().await.with_context(|| "Failed to parse response")
    }

    fn generate_nonce(&self) -> String {
        let mut buf = [0u8; 12];
        let random_bytes: Vec<u8> = thread_rng().gen::<[u8; 8]>().to_vec();
        let random_hex = hex::encode(random_bytes);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i32
            / 60;
        hex::decode_to_slice(random_hex, &mut buf[0..8]).expect("Decoding failed");
        buf[8..].copy_from_slice(&timestamp.to_be_bytes());
        base64::encode(buf)
    }

    fn signed_nonce(&self, secret: &str, nonce: &str) -> String {
        let secret_bytes = base64::decode(secret).expect("Failed to decode secret from base64");
        let nonce_bytes = base64::decode(nonce).expect("Failed to decode nonce from base64");
        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        hasher.update(&nonce_bytes);
        let hash_result = hasher.finalize();
        base64::encode(hash_result)
    }

    fn generate_signature(
        &self,
        path: &str,
        signed_nonce: &str,
        nonce: &str,
        params: &Value,
    ) -> String {
        let mut exps = vec![
            path.to_string(),
            signed_nonce.to_string(),
            nonce.to_string(),
        ];

        if let Some(obj) = params.as_object() {
            let mut param_keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            param_keys.sort();
            for key in param_keys {
                if let Some(value) = obj.get(key) {
                    exps.push(format!(
                        r"{}={}",
                        key,
                        serde_json::to_string(&value).unwrap()
                    ));
                }
            }
        }
        let exps_str = exps.join("&");

        let key = base64::decode(signed_nonce).unwrap();
        let mut signing_key = Hmac::<Sha256>::new_varkey(&key).unwrap();
        signing_key.update(exps_str.as_bytes());
        let result = signing_key.finalize().into_bytes();
        base64::encode(result)
    }

    fn get_api_url(&self, country: &str) -> String {
        match self.urls.to_json().get(country) {
            Some(url) => url.as_str().unwrap().to_string(),
            None => self.urls.cn.clone(),
        }
    }

    fn get_cookie(&self) -> String {
        let mut cookies: Vec<String> = vec![];

        cookies.push("sdkVersion=accountsdk-18.8.15".to_string());
        cookies.push(format!("deviceId={}", self.client_id));
        if let Some(user_id) = &self.user_id.as_ref() {
            cookies.push(format!("userId={}", user_id));
        }
        if let Some(token) = &self.service_token.as_ref() {
            cookies.push(format!("serviceToken={}", token));
            cookies.push(format!("yetAnotherServiceToken={}", token));
        }
        cookies.push(format!("locale={}", self.locale));
        cookies.push("channel=MI_APP_STORE".to_string());

        cookies.join("; ")
    }
}

/// Represents the default state of the MiCloudProtocol.
///
/// This implementation allows creating an instance of `MiCloudProtocol`
/// with default values by using `MiCloudProtocol::default()` or `Default::default()`.
impl Default for MiCloudProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    extern crate tokio;
    use super::*;

    #[test]
    fn signed_nonce() {
        let mi: MiCloudProtocol = MiCloudProtocol::new();
        let result = mi.signed_nonce("9wR21gAtfAyn+KDX1ok/Iw==", "BejIOTLgvecBs9sT");
        let expect = "zq3TaSr/VwnmvvWwMTAEMAuzxs2gLgP6uFJS7bBtWKo=";

        assert_eq!(result, expect);
    }

    #[test]
    fn generate_signature() {
        let mi: MiCloudProtocol = MiCloudProtocol::new();
        let result = mi.generate_signature(
            "/home/device_list",
            "zq3TaSr/VwnmvvWwMTAEMAuzxs2gLgP6uFJS7bBtWKo=",
            "BejIOTLgvecBs9sT",
            &json!({"data": json!({"getVirtualModel":false,"getHuamiDevices":0})}),
        );
        let expect = "6KEUC7sycg/Vhh0Jz7bZqT1JCza7bv36B3WcKnuW9J8=";
        assert_eq!(result, expect);
    }

    #[test]
    fn parse_response_json() {
        let res = super::parse_response_json("&&&START&&&{\"_nonce\":\"BejIOTLgvecBs9sT\",\"data\":{\"getVirtualModel\":false,\"getHuamiDevices\":0}}").unwrap();
        assert_eq!(res["_nonce"], "BejIOTLgvecBs9sT");
        assert_eq!(res["data"]["getVirtualModel"], false);
        assert_eq!(res["data"]["getHuamiDevices"], 0);
    }

    #[test]
    fn serde_value_to_string() {
        let obj = json!({
            "string": "string",
            "bool": true,
            "number": 1
        });
        assert_eq!(super::serde_value_to_string(&obj["string"]), "string");
        assert_eq!(super::serde_value_to_string(&obj["bool"]), "true");
        assert_eq!(super::serde_value_to_string(&obj["number"]), "1");
    }

    #[test]
    fn object_to_query_string() {
        let obj = json!({
            "_nonce": "BejIOTLgvecBs9sT",
            "data": json!({"getVirtualModel":false,"getHuamiDevices":0}),
            "signature": "6KEUC7sycg/Vhh0Jz7bZqT1JCza7bv36B3WcKnuW9J8="
        });
        let result = super::object_to_query_string(&obj);
        let expect = "_nonce=BejIOTLgvecBs9sT&data=%7B%22getHuamiDevices%22%3A0%2C%22getVirtualModel%22%3Afalse%7D&signature=6KEUC7sycg%2FVhh0Jz7bZqT1JCza7bv36B3WcKnuW9J8%3D";
        assert_eq!(result, expect);
    }

    // #[tokio::test]
    async fn e2e() {
        let mut mi: MiCloudProtocol = MiCloudProtocol::new();
        mi._override_urls(UrlsConfig {
            cn: "http://localhost:3000".to_string(),
            de: "http://localhost:3000".to_string(),
            ru: "http://localhost:3000".to_string(),
            sg: "http://localhost:3000".to_string(),
            tw: "http://localhost:3000".to_string(),
            us: "http://localhost:3000".to_string(),
            login_step1: "http://localhost:3000/step1".to_string(),
            login_step2: "http://localhost:3000/step2".to_string(),
        });
        mi.login("username", "password").await.unwrap();
        mi.get_devices(None, None).await.unwrap();
    }
}
