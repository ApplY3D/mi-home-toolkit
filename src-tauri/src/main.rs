// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate anyhow;
extern crate miio;
extern crate serde_json;

use lazy_static::lazy_static;
use miio::{Device, MiCloudProtocol};
use serde_json::Value;
use std::{str::FromStr, sync::Arc};
use tauri_plugin_log::{Builder, Target, TargetKind};
use tokio::sync::Mutex;

lazy_static! {
    static ref MI_CLOUD_PROTOCOL: Arc<Mutex<MiCloudProtocol>> =
        Arc::new(Mutex::new(MiCloudProtocol::new()));
}

//TODO: rm .map_err(|_| ()) https://tauri.app/v1/guides/features/command/#error-handling

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn login(email: String, password: String, country: Option<String>) -> Result<(), String> {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    if let Some(c) = country {
        guard.set_country(&c);
    }
    guard
        .login(email.as_str(), password.as_str())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn set_country(country: String) {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.set_country(&country)
}

#[tauri::command]
async fn get_devices() -> Result<Vec<Device>, ()> {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.get_devices(None, None).await.map_err(|_| ())
}

#[tauri::command]
async fn get_device(did: String) -> Result<Vec<Device>, ()> {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.get_device(&did, None).await.map_err(|_| ())
}

#[tauri::command]
async fn call_device(did: String, method: String, params: Option<String>) -> Result<Value, String> {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    let params = params
        .map(|params| Value::from_str(params.as_str()).map_err(|err| err.to_string()))
        .transpose()?;
    guard
        .call_device(&did, &method, params, None)
        .await
        .map_err(|err| err.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(
            Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            login,
            set_country,
            get_device,
            get_devices,
            call_device
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
