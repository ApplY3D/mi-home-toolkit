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
use regex::Regex;
use reqwest::{
    cookie::{CookieStore, Jar},
    header, Client, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use sha2::{Digest, Sha256};
use std::{
    iter,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
    vec,
};
use uuid::Uuid;

mod async_challenge;
use crate::async_challenge::{AsyncChallengeState, ChallengeSolution};
pub enum CaptchaSolution {
    Solved(String),
    Cancel,
    NotNeeded,
}

enum LoginStep2Result {
    SuccessWithLocation {
        ssecurity: String,
        user_id: i64,
        location: String,
    },
    TwoFactorRequired {
        notification_url: String,
    },
}

enum Handle2FaResult {
    Success {
        ssecurity: String,
        user_id: i64,
        service_token: String,
    },
}

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
    captcha_handler: Option<Box<dyn Fn(String) + Send + Sync>>,
    captcha_state: AsyncChallengeState<String>,
    two_factor_handler: Option<Box<dyn Fn(String, String) + Send + Sync>>,
    two_factor_state: AsyncChallengeState<String>,
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
            captcha_handler: None,
            captcha_state: AsyncChallengeState::<String>::new(),
            two_factor_handler: None,
            two_factor_state: AsyncChallengeState::<String>::new(),
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
        let jar = Arc::new(Jar::default());
        let url = "https://account.xiaomi.com".parse::<Url>().unwrap();
        jar.add_cookie_str(
            &format!("userId={}; deviceId={}", username, self.client_id),
            &url,
        );
        let client = reqwest::Client::builder()
            .cookie_provider(Arc::clone(&jar))
            .build()?;
        let password_md5 = hex_digest(Algorithm::MD5, password.as_bytes()).to_uppercase();
        let step_1_data = self.login_step1(&client, username).await?;
        let sign = match step_1_data["_sign"].as_str() {
            Some(s) => Ok(s.to_string()),
            None => Err(anyhow!("Login step 1 failed: No '_sign' in response")),
        }?;
        let login_step2_res = self
            .login_step2(&client, username, password_md5.as_str(), &sign, None)
            .await?;
        let (ssecurity, user_id, service_token) = match login_step2_res {
            LoginStep2Result::SuccessWithLocation {
                ssecurity,
                user_id,
                location,
            } => {
                let token = self.login_step3(&client, location).await?;
                (ssecurity, user_id, token)
            }
            LoginStep2Result::TwoFactorRequired { notification_url } => {
                println!("notification_url: {:?}", notification_url);
                let two_factor_result = self
                    .handle_2fa(&client, jar.clone(), notification_url)
                    .await?;
                match two_factor_result {
                    Handle2FaResult::Success {
                        ssecurity,
                        user_id,
                        service_token,
                    } => (ssecurity, user_id, service_token),
                }
            }
        };

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

    pub fn _set_captcha_handler(&mut self, handler: Box<dyn Fn(String) + Send + Sync>) {
        self.captcha_handler = Some(handler);
    }

    pub fn _set_two_factor_handler(&mut self, handler: Box<dyn Fn(String, String) + Send + Sync>) {
        self.two_factor_handler = Some(handler);
    }

    async fn login_step1(&self, client: &Client, username: &str) -> Result<Value> {
        let mut captcha: Option<String> = None;
        // recursion in case of captcha
        loop {
            let url = self.urls.login_step1.clone();
            let mut query: Vec<(&'static str, String)> = vec![
                ("sid", "xiaomiio".to_string()),
                ("_json", "true".to_string()),
                ("_locale", "en_US".to_string()),
            ];

            if let Some(c) = captcha.clone() {
                query.push(("captCode", c))
            }

            let res = client
                .get(url)
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
            match self.with_captcha_solving(data.clone()).await {
                CaptchaSolution::Solved(value) => {
                    captcha = Some(value);
                    continue;
                }
                CaptchaSolution::Cancel => {
                    return Err(anyhow!("Captcha cancelled"));
                }
                CaptchaSolution::NotNeeded => {}
            }
            return Ok(data);
        }
    }

    async fn login_step2(
        &self,
        client: &Client,
        username: &str,
        password_md5: &str,
        sign: &str,
        mut captcha: Option<String>,
    ) -> Result<LoginStep2Result> {
        // Loop to handle potential captcha retries.
        loop {
            let form_data = vec![
                ("hash", password_md5.to_string()),
                ("_json", "true".to_string()),
                ("sid", "xiaomiio".to_string()),
                ("callback", "https://sts.api.io.mi.com/sts".to_string()),
                ("qs", "%3Fsid%3Dxiaomiio%26_json%3Dtrue".to_string()),
                ("_sign", sign.to_string()),
                ("user", username.to_string()),
                ("captCode", captcha.unwrap_or("".to_string())),
            ];

            let url = self.urls.login_step2.clone();
            let req = client
                .post(url)
                .form(&form_data)
                .header(header::USER_AGENT, &self.user_agent);

            let res = req.send().await?;

            let status = res.status();
            let content = res.text().await.unwrap_or("".to_string());

            if !status.is_success() {
                return Err(anyhow!(format!(
                    "Login step 2 failed: Response status {}",
                    status
                ),));
            }

            let data = parse_response_json(&content)?;
            // data['code']:
            // 20003 InvalidUserNameException
            // 22009 PackageNameDeniedException
            // 70002 InvalidCredentialException
            // 70016 InvalidCredentialException with captchaUrl / Password error
            // 81003 NeedVerificationException
            // 87001 InvalidResponseException captCode error
            // other NeedCaptchaException

            // Check if a captcha is required.
            match self.with_captcha_solving(data.clone()).await {
                CaptchaSolution::Solved(value) => {
                    captcha = Some(value);
                    continue;
                }
                CaptchaSolution::Cancel => return Err(anyhow!("Captcha cancelled by user")),
                CaptchaSolution::NotNeeded => {}
            }

            // Check for a successful non-2FA login.
            if let Some(ssecurity) = data["ssecurity"].as_str() {
                let user_id = data["userId"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Login step 2 failed: No 'userId'"))?;
                let location = data["location"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Login step 2 failed: No 'location'"))?;
                return Ok(LoginStep2Result::SuccessWithLocation {
                    ssecurity: ssecurity.to_string(),
                    user_id,
                    location: location.to_string(),
                });
            }

            // Check if 2FA is required.
            if let Some(notification_url) = data["notificationUrl"].as_str() {
                return Ok(LoginStep2Result::TwoFactorRequired {
                    notification_url: notification_url.to_string(),
                });
            }

            return Err(anyhow!("Login step 2 failed: No 'ssecurity' or 'notificationUrl' in response. Response: {}",content));
        }
    }

    async fn login_step3(&self, client: &Client, url: String) -> Result<String> {
        client
            .get(url)
            .send()
            .await?
            .cookies()
            .find(|c| c.name() == "serviceToken")
            .map(|c| c.value().to_string())
            .ok_or_else(|| anyhow!("Login Step 3: 'serviceToken' cookie not found in the response"))
    }

    async fn with_captcha_solving(&self, response_json: Value) -> CaptchaSolution {
        if let Some(captcha_url) = response_json["captchaUrl"].as_str() {
            if let Some(handler) = &self.captcha_handler {
                let full_url = format!("https://account.xiaomi.com{captcha_url}");
                let result = self
                    .captcha_state
                    .request_solve(full_url.clone(), |url| async move {
                        handler(url);
                    })
                    .await;

                return match result {
                    Ok(ChallengeSolution::Solved(v)) => CaptchaSolution::Solved(v),
                    _ => CaptchaSolution::Cancel,
                };
            }
        }

        CaptchaSolution::NotNeeded
    }
    pub async fn captcha_solve(&self, value: &str) {
        self.captcha_state.solve(value.to_string()).await
    }

    pub async fn captcha_cancel(&self) {
        self.captcha_state.cancel().await
    }

    /// Handles the entire complex Two-Factor Authentication (2FA) flow.
    ///
    /// This flow is triggered when `login_step2` receives a `notificationUrl`. It involves
    /// requesting a code via email, verifying it, and then carefully following a redirect
    /// chain to extract the `ssecurity` and `serviceToken`. A key part of this flow is
    /// intercepting a redirect to read a custom `extension-pragma` HTTP header.
    async fn handle_2fa(
        &self,
        client: &Client,
        jar: Arc<Jar>,
        notification_url: String,
    ) -> Result<Handle2FaResult> {
        // Step 1: Visit notificationUrl to initialize the session and get initial cookies.
        client
            .get(&notification_url)
            .header(header::USER_AGENT, self.user_agent.to_string())
            .send()
            .await?;

        // Step 2: Extract the 'context' parameter from the notification URL.
        let parsed_url = Url::parse(&notification_url)?;
        let context = parsed_url
            .query_pairs()
            .find_map(|(key, value)| (key == "context").then(|| value.into_owned()))
            .with_context(|| "2FA Flow: Could not find 'context' parameter in notification URL")?;

        // Step 3: Fetch identity options to get the 'identity_session' cookie.
        let list_res = client
            .get("https://account.xiaomi.com/identity/list")
            .query(&[
                ("sid", "xiaomiio"),
                ("context", &context),
                ("supportedMask", "0"),
            ])
            .send()
            .await?
            .error_for_status()?;

        // {"flag":8,"option":8,"options":[8]}
        // "options" - available options for login
        // option 4: '/identity/auth/verifyPhone'
        // option 8: '/identity/auth/verifyEmail'
        let list_text = list_res.text().await?;
        let list_json = parse_response_json(&list_text)?;
        let options = list_json
            .get("options")
            .and_then(|opts| opts.as_array())
            .map(|opts| opts.iter().filter_map(|v| v.as_i64()).collect::<Vec<i64>>())
            .ok_or_else(|| {
                anyhow!("2FA Flow: Could not find 'options' array in identity/list response")
            })?;

        if options.is_empty() {
            return Err(anyhow!(
                    "2FA Flow: Visit <a href=\"{}\" target=\"_blank\"><strong>link</strong></a> to configure account",
                    notification_url
                ));
        }

        // let the user select the option?
        let flag = list_json["flag"]
            .as_i64()
            .ok_or_else(|| anyhow!("2FA Flow: 'flag' not found in 'identity/list' response"))?;

        // Step 4: Request the 2FA code to be sent to the user's email.
        let dc1 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let send_ticket_url = if flag == 4 {
            "https://account.xiaomi.com/identity/auth/sendPhoneTicket"
        } else {
            "https://account.xiaomi.com/identity/auth/sendEmailTicket"
        };
        let send_ticket_res = client
            .post(send_ticket_url)
            .header(header::USER_AGENT, self.user_agent.to_string())
            .query(&[("_dc", &dc1.to_string())])
            .form(&[("retry", "0"), ("icode", ""), ("_json", "true")])
            .send()
            .await?;

        let send_ticket_text = send_ticket_res.text().await?;
        let send_ticket_json = parse_response_json(&send_ticket_text)
            .with_context(|| format!("2FA Flow: Failed to parse {} response", send_ticket_url))?;
        if send_ticket_json.get("code").and_then(Value::as_i64) != Some(0) {
            return Err(anyhow!(
                "2FA Flow: Failed to send 2FA code. Response: {}",
                send_ticket_text
            ));
        }

        // Step 5: Create a separate client WITHOUT automatic redirects.
        // This is crucial for intercepting headers during intermediate steps.
        let no_redirect_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_provider(jar.clone())
            .build()?;

        // Step 6: Loop to get and verify the code from the user.
        let mut error_message = "";
        loop {
            let code = if let Some(handler) = &self.two_factor_handler {
                match self
                    .two_factor_state
                    .request_solve(flag.to_string(), |flag| async move {
                        handler(flag, error_message.to_string());
                    })
                    .await
                {
                    Ok(ChallengeSolution::Solved(code)) => code,
                    _ => return Err(anyhow!("2FA challenge was canceled by the user")),
                }
            } else {
                return Err(anyhow!("2FA is required, but no 2FA handler is configured"));
            };

            // Step 7: Submit the user-provided code for verification.
            let dc2 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
            let verify_url = if flag == 4 {
                "https://account.xiaomi.com/identity/auth/verifyPhone"
            } else {
                "https://account.xiaomi.com/identity/auth/verifyEmail"
            };
            let verify_res = client
                .post(verify_url)
                .header(header::USER_AGENT, self.user_agent.to_string())
                .query(&[("_dc", &dc2.to_string())])
                .form(&[
                    ("_flag", &flag.to_string()),
                    ("ticket", &code),
                    ("trust", &"false".to_string()),
                    ("_json", &"true".to_string()),
                ])
                .send()
                .await?;

            if !verify_res.status().is_success() {
                return Err(anyhow!(
                    "2FA Flow: verify url {} failed with status: {}",
                    verify_url,
                    verify_res.status()
                ));
            }

            let headers = verify_res.headers().clone();
            let res_text = verify_res.text().await?;
            let res_json = parse_response_json(&res_text)?;

            if res_json.get("code").and_then(Value::as_i64) == Some(0) {
                // --- Successful code verification ---

                // Step 8: Reliably extract the URL for the next step ('finish_loc')
                // by checking JSON body, then headers, then regex on body.
                let mut finish_loc = res_json
                    .get("location")
                    .and_then(Value::as_str)
                    .map(String::from);

                if finish_loc.is_none() {
                    finish_loc = headers
                        .get(header::LOCATION)
                        .and_then(|v| v.to_str().ok())
                        .map(String::from);
                }

                if finish_loc.is_none() {
                    let re = Regex::new(
                        r#"https://account\.xiaomi\.com/identity/result/check\?[^"']+"#,
                    )?;
                    finish_loc = re.find(&res_text).map(|m| m.as_str().to_string());
                }

                // Final fallback: directly hit the result/check endpoint.
                let finish_loc = if let Some(loc) = finish_loc {
                    loc
                } else {
                    let fallback_res = no_redirect_client
                        .get("https://account.xiaomi.com/identity/result/check")
                        .query(&[
                            ("sid", "xiaomiio"),
                            ("context", &context),
                            ("_locale", "en_US"),
                        ])
                        .send()
                        .await?;

                    fallback_res
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .map(String::from)
                    .context("2FA Flow: Could not determine next step URL (finish_loc) from any source")?
                };

                // Step 9: Handle the intermediate redirect via 'result/check' if necessary.
                let end_url = if finish_loc.contains("identity/result/check") {
                    no_redirect_client
                        .get(&finish_loc)
                        .send()
                        .await?
                        .headers()
                        .get(header::LOCATION)
                        .and_then(|v| v.to_str().ok())
                        .map(String::from)
                        .context(
                            "2FA Flow: Missing 'Location' header after 'result/check' redirect",
                        )?
                } else {
                    finish_loc
                };

                // Step 10: Request the 'end_url' and handle the optional "tips page" redirect.
                let first_res = no_redirect_client.get(&end_url).send().await?;
                let first_status = first_res.status();
                let mut res_headers = first_res.headers().clone();
                let mut res_text = first_res.text().await?;

                if first_status.is_success() && res_text.contains("Xiaomi Account - Tips") {
                    let second_res = no_redirect_client.get(&end_url).send().await?;
                    res_headers = second_res.headers().clone();
                    res_text = second_res.text().await?;
                }

                // Step 11: Extract 'ssecurity' from the 'extension-pragma' header.
                let ssecurity = res_headers
                    .get("extension-pragma")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| serde_json::from_str::<Value>(s).ok())
                    .and_then(|json| json["ssecurity"].as_str().map(String::from))
                    .context(
                        "2FA Flow: Could not extract 'ssecurity' from 'extension-pragma' header",
                    )?;

                // Step 12: Reliably extract the STS URL from headers or response body.
                let mut sts_url_str = res_headers
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                if sts_url_str.is_none() {
                    if let Some(idx) = res_text.find("https://sts.api.io.mi.com/sts") {
                        let end_idx = res_text[idx..].find('"').unwrap_or(300) + idx;
                        sts_url_str = Some(res_text[idx..end_idx].to_string());
                    }
                }

                let sts_url = sts_url_str
                    .context("2FA Flow: Could not find STS redirect URL in headers or body")?;

                // Step 14: Visit the STS URL to get the 'serviceToken' cookie.
                client.get(&sts_url).send().await?;

                // Step 14: Extract the final 'serviceToken' and 'userId' from the cookie jar.
                let sts_url_parsed = "https://sts.api.io.mi.com".parse::<Url>()?;
                let service_token = jar
                    .cookies(&sts_url_parsed)
                    .and_then(|c| {
                        c.to_str()
                            .ok()
                            .map(String::from)
                            .unwrap_or_default()
                            .split(';')
                            .find_map(|p| p.trim().strip_prefix("serviceToken=").map(String::from))
                    })
                    .context("2FA Flow: Could not find 'serviceToken' cookie")?;

                let user_id_str = jar
                    .cookies(&"https://account.xiaomi.com/".parse::<Url>()?)
                    .and_then(|c| {
                        c.to_str()
                            .ok()
                            .map(String::from)
                            .unwrap_or_default()
                            .split(';')
                            .find_map(|p| p.trim().strip_prefix("userId=").map(String::from))
                    })
                    .context("2FA Flow: Could not find 'userId' cookie")?;

                let user_id = user_id_str.parse::<i64>()?;

                return Ok(Handle2FaResult::Success {
                    ssecurity,
                    user_id,
                    service_token,
                });
            } else if res_json.get("code").and_then(Value::as_i64) == Some(70014) {
                error_message = "Incorrect code. Please try again.";
                continue; // Prompt the user again.
            } else {
                return Err(anyhow!(
                    "2FA Flow: Verification failed with an unexpected error. Response: {}",
                    res_text
                ));
            }
        }
    }

    pub async fn two_factor_solve(&self, value: &str) {
        self.two_factor_state.solve(value.to_string()).await
    }

    pub async fn two_factor_cancel(&self) {
        self.two_factor_state.cancel().await
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
