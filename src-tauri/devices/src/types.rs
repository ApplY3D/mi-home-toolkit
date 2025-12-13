use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "props")]
#[serde(rename_all = "camelCase")]
pub enum ControlStyle {
    Toggle { on: &'static str, off: &'static str },
    Slider { min: i32, max: i32, step: i32 },
    ColorPicker,
}

#[derive(Clone, Serialize)]
pub struct FeatureSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub description: Option<&'static str>,
    pub style: ControlStyle,

    #[serde(skip)]
    pub get_handler: Option<fn() -> Result<(&'static str, Value)>>,
    #[serde(skip)]
    pub set_handler: fn(String) -> Result<(&'static str, Value)>,
}
