use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cross_platform;

const DEFAULT_THRESHOLD_PCT: f64 = 1.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotComparison {
    pub baseline_path: PathBuf,
    pub current_path: PathBuf,
    pub diff_pixels: usize,
    pub total_pixels: usize,
    pub pass_threshold_pct: f64,
}

impl ScreenshotComparison {
    pub fn diff_percentage(&self) -> f64 {
        if self.total_pixels == 0 {
            0.0
        } else {
            (self.diff_pixels as f64 / self.total_pixels as f64) * 100.0
        }
    }

    pub fn passes(&self) -> bool {
        self.diff_percentage() <= self.pass_threshold_pct
    }
}

pub fn current_screenshot_path(project_path: &Path) -> PathBuf {
    project_path
        .join(".lux")
        .join("visual-regression")
        .join("current.rgba")
}

pub fn capture_screenshot_baseline(name: &str, project_path: &Path) -> Result<PathBuf> {
    let source = current_screenshot_path(project_path);
    let baseline_dir = project_path
        .join(".lux")
        .join("visual-regression")
        .join("baselines");
    fs::create_dir_all(&baseline_dir).with_context(|| {
        format!(
            "failed to create {}",
            cross_platform::display_path(&baseline_dir)
        )
    })?;
    let target = baseline_dir.join(format!("{}.rgba", sanitize_name(name)));
    if source.is_file() {
        fs::copy(&source, &target).with_context(|| {
            format!(
                "failed to copy {} to {}",
                cross_platform::display_path(&source),
                cross_platform::display_path(&target)
            )
        })?;
    } else {
        fs::write(&target, []).with_context(|| {
            format!("failed to write {}", cross_platform::display_path(&target))
        })?;
    }
    Ok(cross_platform::normalize_path_buf(target))
}

pub fn compare_screenshots(baseline: &Path, current: &Path) -> ScreenshotComparison {
    let baseline_bytes = fs::read(baseline).unwrap_or_default();
    let current_bytes = fs::read(current).unwrap_or_default();
    let max_len = baseline_bytes.len().max(current_bytes.len());
    let diff_pixels = count_differing_bytes(&baseline_bytes, &current_bytes, max_len);
    ScreenshotComparison {
        baseline_path: cross_platform::normalize_path_buf(baseline.to_path_buf()),
        current_path: cross_platform::normalize_path_buf(current.to_path_buf()),
        diff_pixels,
        total_pixels: max_len,
        pass_threshold_pct: DEFAULT_THRESHOLD_PCT,
    }
}

pub fn pixel_diff_percentage(a: &[u8], b: &[u8], width: usize, height: usize) -> f64 {
    let total_pixels = width.saturating_mul(height);
    if total_pixels == 0 {
        return 0.0;
    }
    let expected_len = total_pixels.saturating_mul(4);
    let diff_pixels = (0usize..total_pixels)
        .filter(|pixel| {
            let start = pixel.saturating_mul(4);
            let end = start + 4;
            if end > expected_len {
                return false;
            }
            let left = a.get(start..end).unwrap_or(&[]);
            let right = b.get(start..end).unwrap_or(&[]);
            left != right
        })
        .count();
    (diff_pixels as f64 / total_pixels as f64) * 100.0
}

fn count_differing_bytes(a: &[u8], b: &[u8], total: usize) -> usize {
    (0..total)
        .filter(|index| a.get(*index) != b.get(*index))
        .count()
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn pixel_diff_identical_images_returns_zero() {
        let image = [0, 0, 0, 255, 255, 255, 255, 255];
        assert_eq!(pixel_diff_percentage(&image, &image, 2, 1), 0.0);
    }

    #[test]
    fn pixel_diff_different_images_returns_positive_percentage() {
        let a = [0, 0, 0, 255, 255, 255, 255, 255];
        let b = [0, 0, 0, 255, 0, 255, 255, 255];
        assert!(pixel_diff_percentage(&a, &b, 2, 1) > 0.0);
    }

    #[test]
    fn screenshot_comparison_passes_below_threshold() {
        let comparison = ScreenshotComparison {
            baseline_path: PathBuf::from("baseline.rgba"),
            current_path: PathBuf::from("current.rgba"),
            diff_pixels: 1,
            total_pixels: 200,
            pass_threshold_pct: 1.0,
        };
        assert!(comparison.passes());
    }

    #[test]
    fn visual_regression_capture_and_compare_known_images() {
        let root = temp_project_root("visual-regression");
        let current = current_screenshot_path(&root);
        fs::create_dir_all(current.parent().unwrap()).unwrap();
        fs::write(&current, [1, 2, 3, 4]).unwrap();
        let baseline = capture_screenshot_baseline("main view", &root).unwrap();
        let comparison = compare_screenshots(&baseline, &current);
        assert_eq!(comparison.diff_pixels, 0);
        assert!(comparison.passes());
        let _ = fs::remove_dir_all(root);
    }

    fn temp_project_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lux-{name}-{nanos}"));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
