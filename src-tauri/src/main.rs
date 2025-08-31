// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate anyhow;
extern crate miio;
extern crate serde_json;

use miio::{Device, MiCloudProtocol};
use serde_json::Value;
use std::{cell::UnsafeCell, mem::MaybeUninit, str::FromStr, sync::Once};
use tauri::{Emitter, Listener};
use tauri_plugin_log::{Builder, Target, TargetKind};

pub static mut MI_CLOUD_PROTOCOL_UNSAFE: MaybeUninit<UnsafeCell<MiCloudProtocol>> =
    MaybeUninit::uninit();
static ONCE: Once = Once::new();

//TODO: rm .map_err(|_| ()) https://tauri.app/v1/guides/features/command/#error-handling

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn login(email: String, password: String, country: Option<String>) -> Result<(), String> {
    unsafe {
        let mut guard = &mut *MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        if let Some(c) = country {
            guard.set_country(&c);
        }
        guard
            .login(email.as_str(), password.as_str())
            .await
            .map_err(|err| err.to_string())
    }
}

#[tauri::command]
async fn get_countries() -> Vec<Vec<&'static str>> {
    unsafe {
        let mut guard = &*MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        guard.get_available_countries()
    }
}

#[tauri::command]
async fn set_country(country: String) {
    unsafe {
        let mut guard = &mut *MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        guard.set_country(&country)
    }
}

#[tauri::command]
async fn get_devices() -> Result<Vec<Device>, ()> {
    unsafe {
        let mut guard = &*MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        guard.get_devices(None, None).await.map_err(|_| ())
    }
}

#[tauri::command]
async fn get_device(did: String) -> Result<Vec<Device>, ()> {
    unsafe {
        let mut guard = &*MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        guard.get_device(&did, None).await.map_err(|_| ())
    }
}

#[tauri::command]
async fn call_device(did: String, method: String, params: Option<String>) -> Result<Value, String> {
    unsafe {
        let mut guard = &*MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
        let params = params
            .map(|params| Value::from_str(params.as_str()).map_err(|err| err.to_string()))
            .transpose()?;
        guard
            .call_device(&did, &method, params, None)
            .await
            .map_err(|err| err.to_string())
    }
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
            get_countries,
            set_country,
            get_device,
            get_devices,
            call_device
        ])
        .setup(|app| {
            ONCE.call_once(|| unsafe {
                MI_CLOUD_PROTOCOL_UNSAFE.write(UnsafeCell::new(MiCloudProtocol::new()));
            });

            let app_handle = app.handle();

            tauri::async_runtime::spawn({
                let app_handle = app_handle.clone();
                async move {
                    unsafe {
                        let mut guard = &mut *MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
                        guard._set_captcha_handler(Box::new(move |x| {
                            let app_handle = app_handle.clone();
                            let _ = app_handle.emit("captcha_requested", x);
                        }));
                    }
                }
            });

            app_handle.listen("captcha_solved", move |event| {
                tauri::async_runtime::spawn(async move {
                    unsafe {
                        let guard = &*MI_CLOUD_PROTOCOL_UNSAFE.assume_init_ref().get();
                        let pl = serde_json::from_str::<String>(event.payload()).unwrap();
                        if pl.eq("CANCEL") {
                            guard.captcha_cancel().await
                        } else {
                            guard.captcha_solve(pl.as_str()).await
                        }
                    }
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
