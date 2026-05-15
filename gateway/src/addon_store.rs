use crate::addon_auth::RepoVisibility;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonEntry {
    pub id: String,
    pub name: String,
    pub repo_url: String,
    pub version: String,
    pub description: String,
    pub auth_status: String,
    pub accessible: bool,
    pub visibility: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedToken {
    pub token: String,
    pub repos: Vec<String>,
    pub expires_at: u64,
}

#[derive(Default)]
pub struct AddonStore {
    addons: HashMap<String, AddonEntry>,
}

impl AddonStore {
    pub fn new() -> Self {
        Self {
            addons: HashMap::new(),
        }
    }

    pub fn register(&mut self, addon: AddonEntry) {
        self.addons.insert(addon.id.clone(), addon);
    }

    pub fn unregister(&mut self, id: &str) -> Option<AddonEntry> {
        self.addons.remove(id)
    }

    pub fn list(&self) -> Vec<AddonEntry> {
        self.addons.values().cloned().collect()
    }

    pub fn list_public(&self) -> Vec<AddonEntry> {
        self.addons
            .values()
            .filter(|a| a.visibility == "public")
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<AddonEntry> {
        self.addons.get(id).cloned()
    }

    pub fn get_by_name(&self, name: &str) -> Option<AddonEntry> {
        self.addons.values().find(|a| a.name == name).cloned()
    }

    pub fn update_auth_status(&mut self, id: &str, status: String, accessible: bool) -> bool {
        if let Some(addon) = self.addons.get_mut(id) {
            addon.auth_status = status;
            addon.accessible = accessible;
            true
        } else {
            false
        }
    }

    pub fn set_visibility(&mut self, id: &str, visibility: RepoVisibility) -> bool {
        if let Some(addon) = self.addons.get_mut(id) {
            addon.visibility = match visibility {
                RepoVisibility::Public => "public".to_string(),
                RepoVisibility::Private => "private".to_string(),
                RepoVisibility::NotFound => "unknown".to_string(),
            };
            true
        } else {
            false
        }
    }
}

pub const KNOWN_LINALAB_PACKAGES: &[&str] = &[
    "com.linalab.lux",
    "com.linalab.unity-log",
    "com.linalab.easy-fps",
    "com.linalab.easy-map-editor",
    "com.linalab.unitybase.core",
    "com.linalab.unitybase.steamworks",
    "com.linalab.unity-codex-image",
];

pub fn discover_linalab_packages(packages_dir: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(packages_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("com.linalab.") && KNOWN_LINALAB_PACKAGES.contains(&name) {
                    found.push(name.to_string());
                }
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addon_store() {
        let mut store = AddonStore::new();
        let addon = AddonEntry {
            id: "test-addon".to_string(),
            name: "Test Addon".to_string(),
            repo_url: "https://github.com/linalab/test-addon".to_string(),
            version: "1.0.0".to_string(),
            description: "A test addon".to_string(),
            auth_status: "unverified".to_string(),
            accessible: false,
            visibility: "unknown".to_string(),
        };

        store.register(addon.clone());
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.get("test-addon").unwrap().name, "Test Addon");

        store.update_auth_status("test-addon", "verified".to_string(), true);
        assert_eq!(store.get("test-addon").unwrap().auth_status, "verified");
        assert!(store.get("test-addon").unwrap().accessible);

        store.unregister("test-addon");
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_set_visibility() {
        let mut store = AddonStore::new();
        let addon = AddonEntry {
            id: "vis-test".to_string(),
            name: "Vis Test".to_string(),
            repo_url: "https://github.com/linalab/vis-test".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            auth_status: "unverified".to_string(),
            accessible: false,
            visibility: "unknown".to_string(),
        };
        store.register(addon);

        assert!(store.set_visibility("vis-test", RepoVisibility::Public));
        assert_eq!(store.get("vis-test").unwrap().visibility, "public");

        assert!(store.set_visibility("vis-test", RepoVisibility::Private));
        assert_eq!(store.get("vis-test").unwrap().visibility, "private");

        assert!(!store.set_visibility("nonexistent", RepoVisibility::Public));
    }

    #[test]
    fn test_list_public_filters_correctly() {
        let mut store = AddonStore::new();
        for (id, vis) in [("a1", "public"), ("a2", "private"), ("a3", "public")] {
            store.register(AddonEntry {
                id: id.to_string(),
                name: id.to_string(),
                repo_url: format!("https://github.com/linalab/{}", id),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                auth_status: "unverified".to_string(),
                accessible: false,
                visibility: vis.to_string(),
            });
        }

        let public = store.list_public();
        assert_eq!(public.len(), 2);
        assert!(public.iter().all(|a| a.visibility == "public"));
    }

    #[test]
    fn test_get_by_name() {
        let mut store = AddonStore::new();
        store.register(AddonEntry {
            id: "id-1".to_string(),
            name: "com.linalab.lux".to_string(),
            repo_url: "https://github.com/linalab/com.linalab.lux".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            auth_status: "unverified".to_string(),
            accessible: false,
            visibility: "unknown".to_string(),
        });

        assert!(store.get_by_name("com.linalab.lux").is_some());
        assert!(store.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_discover_linalab_packages() {
        let dir = std::env::temp_dir().join(format!("lux-discover-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("com.linalab.lux")).unwrap();
        std::fs::create_dir_all(dir.join("com.linalab.unity-log")).unwrap();
        std::fs::create_dir_all(dir.join("com.other.package")).unwrap();
        std::fs::create_dir_all(dir.join("not-a-package")).unwrap();

        let found = discover_linalab_packages(&dir);
        assert_eq!(found, vec!["com.linalab.lux", "com.linalab.unity-log"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_known_packages_constant() {
        assert_eq!(KNOWN_LINALAB_PACKAGES.len(), 7);
        assert!(KNOWN_LINALAB_PACKAGES.contains(&"com.linalab.lux"));
    }
}
