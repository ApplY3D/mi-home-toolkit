use crate::{
    features::{FEAT_BRIGHT, FEAT_LAN, FEAT_POWER, FEAT_RGB},
    FeatureSpec,
};

pub fn resolve(model: &str) -> Vec<&'static FeatureSpec> {
    let mut features = Vec::new();

    if model.starts_with("yeelink.light") {
        features.push(&FEAT_POWER);
        let is_monochrome = model.contains("mono") || model.contains("ceiling");
        if !is_monochrome {
            features.push(&FEAT_RGB);
        }
        features.push(&FEAT_BRIGHT);
        features.push(&FEAT_LAN);
    }

    features
}
