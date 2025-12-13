use crate::types::{ControlStyle, FeatureSpec};
use serde_json::json;

pub const FEAT_LAN: FeatureSpec = {
    fn get() -> anyhow::Result<(&'static str, serde_json::Value)> {
        Ok(("get_prop", json!(["lan_ctrl"])))
    }
    fn set(val: String) -> anyhow::Result<(&'static str, serde_json::Value)> {
        let v = match val.as_str() {
            "1" | "true" | "on" => "1",
            _ => "0",
        };
        Ok(("set_ps", json!(["cfg_lan_ctrl", v])))
    }
    FeatureSpec {
        id: "lan_mode",
        label: "LAN Control",
        description: None,
        style: ControlStyle::Toggle { on: "1", off: "0" },
        get_handler: Some(get),
        set_handler: set,
    }
};

pub const FEAT_POWER: FeatureSpec = {
    fn get() -> anyhow::Result<(&'static str, serde_json::Value)> {
        Ok(("get_prop", json!(["power"])))
    }
    fn set(val: String) -> anyhow::Result<(&'static str, serde_json::Value)> {
        let v = match val.as_str() {
            "1" | "true" | "on" => "on",
            _ => "off",
        };
        Ok(("set_power", json!([v, "smooth", 500])))
    }
    FeatureSpec {
        id: "power",
        label: "Power",
        description: None,
        style: ControlStyle::Toggle { on: "1", off: "0" },
        get_handler: Some(get),
        set_handler: set,
    }
};

pub const FEAT_BRIGHT: FeatureSpec = {
    fn get() -> anyhow::Result<(&'static str, serde_json::Value)> {
        Ok(("get_prop", json!(["bright"])))
    }
    fn set(val: String) -> anyhow::Result<(&'static str, serde_json::Value)> {
        Ok((
            "set_bright",
            json!([val.parse().unwrap_or(50).clamp(1, 100), "smooth", 500]),
        ))
    }
    FeatureSpec {
        id: "bright",
        label: "Brightness",
        description: None,
        style: ControlStyle::Slider {
            min: 1,
            max: 100,
            step: 1,
        },
        get_handler: Some(get),
        set_handler: set,
    }
};

pub const FEAT_RGB: FeatureSpec = {
    fn get() -> anyhow::Result<(&'static str, serde_json::Value)> {
        Ok(("get_prop", json!(["rgb"])))
    }
    fn set(val: String) -> anyhow::Result<(&'static str, serde_json::Value)> {
        // HEX (#FFFFFF, ff0000, FF0000) and INT (16711680)
        let v_str = val.trim_start_matches('#').trim_start_matches("0x");
        let v = u32::from_str_radix(v_str, 16).unwrap_or(0);
        Ok(("set_rgb", json!([v, "smooth", 500])))
    }
    FeatureSpec {
        id: "rgb",
        label: "RGB Color",
        description: None,
        style: ControlStyle::ColorPicker,
        get_handler: Some(get),
        set_handler: set,
    }
};
