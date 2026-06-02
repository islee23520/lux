use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    lux_manual_qa::ManualQaEvidenceRequest,
    lux_manual_qa_labels::{evidence_label, path_to_slash},
};

pub(crate) fn write_manual_qa_evidence(
    request: &ManualQaEvidenceRequest,
    label: &str,
    payload: Value,
) -> Result<String> {
    let relative_dir = relative_evidence_dir(request);
    let absolute_dir = request.project_path.join(&relative_dir);
    fs::create_dir_all(&absolute_dir).with_context(|| {
        format!(
            "failed to create manual QA evidence directory {}",
            absolute_dir.display()
        )
    })?;
    let file_name = format!("manual_qa_{}.json", evidence_label(label));
    let absolute_path = absolute_dir.join(&file_name);
    let temp_path = absolute_path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(&payload).context("failed to serialize evidence")?;
    fs::write(&temp_path, bytes)
        .with_context(|| format!("failed to write temp evidence {}", temp_path.display()))?;
    fs::rename(&temp_path, &absolute_path).with_context(|| {
        format!(
            "failed to atomically replace manual QA evidence {}",
            absolute_path.display()
        )
    })?;
    Ok(format!("{}/{}", path_to_slash(&relative_dir), file_name))
}

fn relative_evidence_dir(request: &ManualQaEvidenceRequest) -> std::path::PathBuf {
    if request.evidence_dir.is_absolute() {
        return request
            .evidence_dir
            .strip_prefix(&request.project_path)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(format!(".lux/evidence/manual-qa/{}", request.run_id))
            });
    }
    request.evidence_dir.clone()
}
