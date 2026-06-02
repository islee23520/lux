use std::fs;
use std::path::Path;

pub fn read_skill_references(directory_path: &Path) -> Vec<String> {
    let references_dir = directory_path.join("references");
    let mut references = Vec::new();
    collect_markdown_references(&references_dir, &references_dir, &mut references);
    references.sort();
    references
}

pub fn read_skill_md_preview(directory_path: &Path) -> Vec<String> {
    let skill_md_path = directory_path.join("SKILL.md");
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    content.lines().take(10).map(str::to_string).collect()
}

fn collect_markdown_references(root: &Path, directory: &Path, references: &mut Vec<String>) {
    let Ok(read_dir) = fs::read_dir(directory) else {
        return;
    };
    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if path.is_dir() {
            collect_markdown_references(root, &path, references);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        references.push(relative.to_string_lossy().replace('\\', "/"));
    }
}
