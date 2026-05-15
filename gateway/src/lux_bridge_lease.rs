use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::lux_io::atomic_write_json;

/// Bridge lease — serializes Unity bridge access across concurrent agents.
/// Read leases are concurrent; Write lease is exclusive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeLease {
    pub id: String,
    pub agent_id: String,
    pub kind: LeaseKind,
    pub acquired_at: String,
    pub expires_at: String,
    pub purpose: String,
    pub status: LeaseStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaseKind {
    Read,
    Write,
    Play,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaseStatus {
    Active,
    Released,
    Expired,
    Revoked,
}

/// Lease queue configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseQueueConfig {
    pub default_ttl_secs: u64,
    pub max_queue_depth: u32,
    pub play_blocks_everything: bool,
}

impl Default for LeaseQueueConfig {
    fn default() -> Self {
        Self {
            default_ttl_secs: 300,
            max_queue_depth: 32,
            play_blocks_everything: true,
        }
    }
}

impl BridgeLease {
    pub fn acquire(
        lux_dir: &Path,
        agent_id: &str,
        kind: LeaseKind,
        purpose: &str,
        ttl_secs: u64,
        config: &LeaseQueueConfig,
    ) -> Result<BridgeLease> {
        expire_stale_leases(lux_dir)?;
        let active = list_active_leases(lux_dir)?;
        if let Some(conflict) = Self::check_conflicts(&active, &kind, config)? {
            bail!(conflict);
        }

        let ttl = if ttl_secs == 0 {
            config.default_ttl_secs
        } else {
            ttl_secs
        };
        let now = Utc::now();
        let lease = BridgeLease {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            kind,
            acquired_at: now.to_rfc3339(),
            expires_at: (now + Duration::seconds(ttl as i64)).to_rfc3339(),
            purpose: purpose.to_string(),
            status: LeaseStatus::Active,
        };
        write_lease(lux_dir, &lease)?;
        Ok(lease)
    }

    pub fn release(&self, lux_dir: &Path) -> Result<()> {
        let mut released = self.clone();
        released.status = LeaseStatus::Released;
        write_lease(lux_dir, &released)
    }

    pub fn renew(&mut self, ttl_secs: u64) -> Result<()> {
        if self.status != LeaseStatus::Active {
            bail!(
                "Cannot renew bridge lease {} with status {:?}",
                self.id,
                self.status
            );
        }
        if is_expired(self)? {
            self.status = LeaseStatus::Expired;
            bail!("Cannot renew expired bridge lease {}", self.id);
        }
        self.expires_at = (Utc::now() + Duration::seconds(ttl_secs as i64)).to_rfc3339();
        Ok(())
    }

    pub fn renew_persisted(&mut self, lux_dir: &Path, ttl_secs: u64) -> Result<()> {
        self.renew(ttl_secs)?;
        write_lease(lux_dir, self)
    }

    pub fn check_conflicts(
        existing_leases: &[BridgeLease],
        requested_kind: &LeaseKind,
        config: &LeaseQueueConfig,
    ) -> Result<Option<String>> {
        for lease in existing_leases {
            if lease.status != LeaseStatus::Active || is_expired(lease)? {
                continue;
            }

            let conflict = match requested_kind {
                LeaseKind::Read => config.play_blocks_everything && lease.kind == LeaseKind::Play,
                LeaseKind::Write => {
                    lease.kind == LeaseKind::Write
                        || (config.play_blocks_everything && lease.kind == LeaseKind::Play)
                }
                LeaseKind::Play => {
                    if config.play_blocks_everything {
                        true
                    } else {
                        lease.kind == LeaseKind::Write || lease.kind == LeaseKind::Play
                    }
                }
            };

            if conflict {
                return Ok(Some(format!(
                    "Bridge lease conflict: requested {:?} lease is blocked by active {:?} lease {} from agent {} for {}",
                    requested_kind, lease.kind, lease.id, lease.agent_id, lease.purpose
                )));
            }
        }

        Ok(None)
    }
}

pub fn expire_stale_leases(lux_dir: &Path) -> Result<usize> {
    let mut expired = 0;
    for mut lease in read_all_leases(lux_dir)? {
        if lease.status == LeaseStatus::Active && is_expired(&lease)? {
            lease.status = LeaseStatus::Expired;
            write_lease(lux_dir, &lease)?;
            expired += 1;
        }
    }
    Ok(expired)
}

pub fn active_write_lease_exists(lux_dir: &Path) -> Result<Option<BridgeLease>> {
    expire_stale_leases(lux_dir)?;
    Ok(list_active_leases(lux_dir)?
        .into_iter()
        .find(|lease| lease.kind == LeaseKind::Write || lease.kind == LeaseKind::Play))
}

pub fn list_active_leases(lux_dir: &Path) -> Result<Vec<BridgeLease>> {
    let leases = read_all_leases(lux_dir)?
        .into_iter()
        .filter(|lease| lease.status == LeaseStatus::Active)
        .filter(|lease| !matches!(is_expired(lease), Ok(true)))
        .collect();
    Ok(leases)
}

fn read_all_leases(lux_dir: &Path) -> Result<Vec<BridgeLease>> {
    let dir = lease_dir(lux_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut leases = Vec::new();
    for entry in
        fs::read_dir(&dir).with_context(|| format!("reading bridge lease dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading bridge lease {}", path.display()))?;
        let lease: BridgeLease = serde_json::from_str(&text)
            .with_context(|| format!("parsing bridge lease {}", path.display()))?;
        leases.push(lease);
    }
    Ok(leases)
}

fn write_lease(lux_dir: &Path, lease: &BridgeLease) -> Result<()> {
    let path = lease_dir(lux_dir).join(format!("{}.json", lease.id));
    atomic_write_json(&path, lease)
}

fn lease_dir(lux_dir: &Path) -> std::path::PathBuf {
    lux_dir.join("bridge-leases")
}

fn is_expired(lease: &BridgeLease) -> Result<bool> {
    let expires_at = DateTime::parse_from_rfc3339(&lease.expires_at)
        .with_context(|| format!("bridge lease {} has invalid expires_at", lease.id))?
        .with_timezone(&Utc);
    Ok(expires_at <= Utc::now())
}
