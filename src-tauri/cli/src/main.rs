use anyhow::Result;
use mi_home_devices::{get_features_for_model, ControlStyle, FeatureSpec};
use miio::{Device, MiCloudProtocol};
use serde_json::Value;
use std::io::{self, Write};

// --- Global State ---
static mut APP: Option<MiCloudProtocol> = None;

fn app() -> &'static mut MiCloudProtocol {
    unsafe { APP.as_mut().expect("App not initialized") }
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

// --- Main Entry Point ---
#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        APP = Some(MiCloudProtocol::new());
    }

    setup_callbacks();
    let mi = app();

    println!("--- Mi Home Toolkit CLI ---");

    select_server(mi);
    perform_login(mi).await;

    device_selection_loop(mi).await?;

    Ok(())
}

// --- Setup & Auth Logic ---
fn setup_callbacks() {
    let rt = tokio::runtime::Handle::current();

    let rt_c = rt.clone();
    app()._set_captcha_handler(Box::new(move |url| {
        println!("\n[!] CAPTCHA required: {}", url);
        let rt = rt_c.clone();
        tokio::task::spawn_blocking(move || {
            let code = get_input("Enter CAPTCHA (or 'CANCEL'): ");
            rt.block_on(async {
                if code == "CANCEL" {
                    app().captcha_cancel().await;
                } else {
                    app().captcha_solve(&code).await;
                }
            });
        });
    }));

    let rt_2fa = rt.clone();
    app()._set_two_factor_handler(Box::new(move |flag, msg| {
        let method = if flag == "8" { "email" } else { "phone" };
        println!("\n[!] 2FA Code sent to {}. Msg: {}", method, msg);

        let rt = rt_2fa.clone();
        tokio::task::spawn_blocking(move || {
            let code = get_input("Enter 2FA code (or 'CANCEL'): ");
            rt.block_on(async {
                if code == "CANCEL" {
                    app().two_factor_cancel().await;
                } else {
                    app().two_factor_solve(&code).await;
                }
            });
        });
    }));
}

fn select_server(mi: &mut MiCloudProtocol) {
    let countries = mi.get_available_countries();
    println!("\nAvailable Servers:");
    for (i, c) in countries.iter().enumerate() {
        println!("{}. {} ({})", i + 1, c[1], c[0]);
    }

    loop {
        let choice = get_input("Select server (default 1): ");
        let idx = choice.parse::<usize>().unwrap_or(1);

        if idx > 0 && idx <= countries.len() {
            mi.set_country(countries[idx - 1][0]);
            break;
        }
        println!("Invalid selection.");
    }
}

async fn perform_login(mi: &mut MiCloudProtocol) {
    loop {
        let username = get_input("\nUsername: ");
        if username.trim().is_empty() {
            continue;
        }

        let password =
            rpassword::prompt_password("Password: ").unwrap_or_else(|_| get_input("Password: "));

        println!("Logging in...");
        match mi.login(&username, &password).await {
            Ok(_) => {
                println!("Success!");
                break;
            }
            Err(e) => eprintln!("\n[!] Login error: {}\nPlease try again.", e),
        }
    }
}

// --- Device Selection Loop ---
async fn device_selection_loop(mi: &mut MiCloudProtocol) -> Result<()> {
    loop {
        println!("\nFetching devices...");
        let devices = match mi.get_devices(None, None).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("\n[!] Error fetching devices: {}", e);
                if get_input("Retry? (Y/n): ").to_lowercase() == "n" {
                    break;
                }
                continue;
            }
        };

        if devices.is_empty() {
            println!("No devices found.");
            if get_input("Check another server? (Y/n): ").to_lowercase() == "n" {
                break;
            }
            select_server(mi);
            continue;
        }

        println!("\nDevices:");
        for (i, d) in devices.iter().enumerate() {
            println!(
                "{}. {} {} (Model: {}, IP: {})",
                i + 1,
                if d.isOnline {
                    "(\x1b[32m●\x1b[0m)"
                } else {
                    "(○)"
                },
                d.name,
                d.model,
                d.localip
            );
        }
        println!("0. Exit");

        let choice = get_input("\nSelect device > ");
        if choice == "0" {
            break;
        }

        if let Ok(idx) = choice.parse::<usize>() {
            if idx > 0 && idx <= devices.len() {
                let selected_device = devices[idx - 1].clone();
                device_control_loop(mi, selected_device).await;
            } else {
                println!("Invalid index.");
            }
        }
    }
    Ok(())
}

async fn device_control_loop(mi: &mut MiCloudProtocol, dev: Device) {
    loop {
        println!("\n--- {} ({}) ---", dev.name, dev.model);
        println!("1. Info");
        println!("2. RPC Command / Features");
        println!("0. Back");

        match get_input("Action > ").as_str() {
            "1" => match serde_json::to_string_pretty(&dev) {
                Ok(j) => println!("{}", j),
                Err(_) => println!("{:#?}", dev),
            },
            "2" => perform_rpc_action_loop(mi, &dev).await,
            "0" => break,
            _ => println!("Invalid option."),
        }
    }
}

// --- Feature / RPC Logic ---
async fn perform_rpc_action_loop(mi: &mut MiCloudProtocol, dev: &Device) {
    let features = get_features_for_model(&dev.model);

    if features.is_empty() {
        println!("(No custom features available for this model)");
        return;
    }

    println!("\nHint: You can just type '1' to GET (short for 11).");

    loop {
        print_features_menu(dev, &features);

        let input =
            get_input("\nFeature Code (11) or Raw Method (get_prop). Empty or 0 to back > ");
        if input.trim().is_empty() || input.trim() == "0" {
            break;
        }
        // Feature Code
        if let Some((feat_idx, action_char)) = try_parse_feature_code(&input) {
            if let Some(feat) = features.get(feat_idx) {
                execute_feature_action(mi, dev, feat, action_char).await;
                continue;
            }
        }

        // Raw Method
        println!("Treating '{}' as raw method name.", input);
        let params = ask_for_json_params();
        send_rpc(mi, &dev.did, &input, params).await;

        break;
    }
}

fn print_features_menu(dev: &Device, features: &[&FeatureSpec]) {
    println!("\n--- Available Actions ({}) ---", dev.model);
    println!("{:<6} | {:<25}", "Code", "Action");
    println!("{:-<6}-+-{:-<25}", "", "");

    for (i, f) in features.iter().enumerate() {
        let idx = i + 1;
        if f.get_handler.is_some() {
            println!("{:<6} | GET {:<21}", idx * 10 + 1, f.label,);
        }
        println!("{:<6} | SET {:<21}", idx * 10 + 2, f.label,);
    }
}

fn try_parse_feature_code(input: &str) -> Option<(usize, char)> {
    if !input.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    if input.len() == 1 {
        let idx_plus_1 = input.parse::<usize>().ok()?;
        Some((idx_plus_1 - 1, '1'))
    } else {
        // "102" -> Index "10", Action '2'
        let split_pos = input.len() - 1;
        let index_part = &input[..split_pos];
        let action_char = input.chars().last()?;
        let idx_plus_1 = index_part.parse::<usize>().ok()?;
        if idx_plus_1 == 0 {
            return None;
        }
        Some((idx_plus_1 - 1, action_char))
    }
}

async fn execute_feature_action(
    mi: &mut MiCloudProtocol,
    dev: &Device,
    feat: &FeatureSpec,
    action: char,
) {
    match action {
        '1' => {
            // GET
            if let Some(getter) = feat.get_handler {
                match getter() {
                    Ok((m, p)) => send_rpc(mi, &dev.did, m, Some(p)).await,
                    Err(e) => eprintln!("Error preparing GET: {}", e),
                }
            } else {
                println!("Feature '{}' has no GET handler.", feat.label);
            }
        }
        '2' => {
            // SET
            if let Some(val) = get_smart_feature_input(feat) {
                match (feat.set_handler)(val) {
                    Ok((m, p)) => send_rpc(mi, &dev.did, m, Some(p)).await,
                    Err(e) => eprintln!("Error preparing SET: {}", e),
                }
            } else {
                println!("Cancelled.");
            }
        }
        _ => println!("Invalid suffix '{}'. Use 1 (Get) or 2 (Set).", action),
    }
}

fn get_smart_feature_input(feat: &FeatureSpec) -> Option<String> {
    println!("\n--- Setting: {} ---", feat.label);

    loop {
        match feat.style {
            ControlStyle::Toggle { on, off } => {
                let raw = get_input("Enable? [Y/n]: ");
                match raw.trim().to_lowercase().as_str() {
                    "y" | "yes" | "true" | "1" | "on" | "" => return Some(on.to_string()),
                    "n" | "no" | "false" | "0" | "off" => return Some(off.to_string()),
                    "cancel" | "q" => return None,
                    _ => println!("Please enter 'y' or 'n'."),
                }
            }
            ControlStyle::Slider { min, max, .. } => {
                let raw = get_input(&format!("Enter value ({}-{}): ", min, max));
                if raw.trim().to_lowercase() == "cancel" {
                    return None;
                }
                match raw.trim().parse::<i32>() {
                    Ok(num) if num >= min && num <= max => return Some(raw),
                    Ok(_) => println!("Value out of range!"),
                    Err(_) => println!("Please enter a valid number."),
                }
            }
            ControlStyle::ColorPicker => {
                println!("Formats: Hex (#FFFFFF) or Int (16711680)");
                let raw = get_input("Enter color: ");
                if raw.trim().is_empty() || raw == "cancel" {
                    return None;
                }

                let clean = raw.trim().trim_start_matches('#').trim_start_matches("0x");
                if u32::from_str_radix(clean, 16).is_ok() {
                    return Some(raw);
                }
                println!("Invalid color format.");
            }
            _ => return Some(get_input("Enter value: ")),
        }
    }
}

async fn send_rpc(mi: &mut MiCloudProtocol, did: &str, method: &str, params: Option<Value>) {
    println!("Sending RPC: {} params: {:?}", method, params);
    match mi.call_device(did, method, params, None).await {
        Ok(res) => println!(
            "Result: {}",
            serde_json::to_string_pretty(&res).unwrap_or(format!("{:?}", res))
        ),
        Err(e) => eprintln!("[!] RPC Failed: {}", e),
    }
}

fn ask_for_json_params() -> Option<Value> {
    let params_raw = get_input("Params JSON (e.g. [\"val\"] or []): ");
    if params_raw.trim().is_empty() {
        return None;
    }
    match serde_json::from_str(&params_raw) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("Invalid JSON format: {}", e);
            None
        }
    }
}
