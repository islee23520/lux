use crate::SUPPORTED_SPEC_MAJOR_VERSION;

pub fn validate_supported_version(version: &str) -> Result<(), String> {
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next();
    let patch = parts.next();

    if parts.next().is_some()
        || major != SUPPORTED_SPEC_MAJOR_VERSION
        || minor.and_then(|part| part.parse::<u64>().ok()).is_none()
        || patch.and_then(|part| part.parse::<u64>().ok()).is_none()
    {
        return Err(format!("unsupported spec version: {version}"));
    }

    Ok(())
}

pub(crate) fn default_glossary_path() -> String {
    "glossary.md".to_string()
}

pub fn validate_score(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0.0 and 1.0"));
    }
    Ok(())
}

pub fn clamp_score(value: f64) -> f64 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}
