//! Dépôt d'objets immuables (hash BLAKE3) + manifestes complets (ADR 0002).
//! Layout : repo.json, objects/ab/<reste>, snapshots/<id>.manifest, latest_success, tmp/.
pub mod backup;
pub mod restore;

use core_types::{check_schema, Event};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const REPO_FORMAT: u32 = 1;
pub const MANIFEST_SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize)]
struct RepoMeta { format_version: u32, repo_id: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus { Succeeded, Partial, Interrupted }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryKind { File { hash: String }, Dir, Symlink { target: String } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry { pub path: String, #[serde(flatten)] pub kind: EntryKind, pub size: u64, pub mtime_ns: i128, pub mode: u32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub created_unix: u64,
    pub status: SnapshotStatus,
    pub entries: Vec<ManifestEntry>,
    pub errors: Vec<Event>,
    pub warnings: Vec<Event>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode { Quick, Full }

pub struct Repo { pub root: PathBuf }

/// Verrou exclusif du dépôt (flock) : libéré automatiquement à la fin du processus, même après crash.
pub struct RepoLock(#[allow(dead_code)] std::fs::File);

pub(crate) fn io_err(code: &str, p: &Path, e: &std::io::Error) -> Event {
    Event::new(code).arg("path", p.display()).arg("kind", e.kind())
}

/// Écriture atomique : fichier temporaire dans le même dossier, fsync, rename.
pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp-write");
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(data)?; f.sync_all()?;
    std::fs::rename(&tmp, path)
}

impl Repo {
    /// Ouvre ou initialise un dépôt. Refuse un format futur inconnu.
    pub fn open_or_init(root: &Path) -> Result<Repo, Event> {
        let meta = root.join("repo.json");
        if meta.exists() {
            let m: RepoMeta = serde_json::from_slice(&std::fs::read(&meta).map_err(|e| io_err("repo.unreadable", &meta, &e))?)
                .map_err(|_| Event::new("repo.corrupt_meta"))?;
            check_schema(m.format_version, REPO_FORMAT)?;
        } else {
            for d in ["objects", "snapshots", "tmp"] {
                std::fs::create_dir_all(root.join(d)).map_err(|e| io_err("repo.init_failed", root, &e))?;
            }
            let id = blake3::hash(format!("{:?}{}", std::time::SystemTime::now(), std::process::id()).as_bytes()).to_hex().to_string();
            let m = RepoMeta { format_version: REPO_FORMAT, repo_id: id[..16].into() };
            atomic_write(&meta, &serde_json::to_vec_pretty(&m).unwrap()).map_err(|e| io_err("repo.init_failed", &meta, &e))?;
        }
        Ok(Repo { root: root.to_path_buf() })
    }
    /// Un seul écrivain à la fois. Sous verrou, nettoie les résidus d'un arrêt brutal (tmp/, *.tmp-write).
    pub fn lock(&self) -> Result<RepoLock, Event> {
        let p = self.root.join("lock");
        let f = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&p).map_err(|e| io_err("repo.lock_failed", &p, &e))?;
        match f.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Err(Event::new("repo.locked")),
            Err(std::fs::TryLockError::Error(e)) => return Err(io_err("repo.lock_failed", &p, &e)),
        }
        for e in std::fs::read_dir(self.root.join("tmp")).into_iter().flatten().flatten() { let _ = std::fs::remove_file(e.path()); }
        for d in [self.root.clone(), self.root.join("snapshots")] {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                if e.path().extension().is_some_and(|x| x == "tmp-write") { let _ = std::fs::remove_file(e.path()); }
            }
        }
        Ok(RepoLock(f))
    }
    pub fn object_path(&self, hash: &str) -> PathBuf { self.root.join("objects").join(&hash[..2]).join(&hash[2..]) }
    pub fn manifest_path(&self, id: &str) -> PathBuf { self.root.join("snapshots").join(format!("{id}.manifest")) }

    pub fn load_manifest(&self, id: &str) -> Result<Manifest, Event> {
        let p = self.manifest_path(id);
        let b = std::fs::read(&p).map_err(|e| io_err("snapshot.unreadable", &p, &e))?;
        // Lire d'abord la version seule pour refuser proprement un futur format.
        let v: serde_json::Value = serde_json::from_slice(&b).map_err(|_| Event::new("snapshot.corrupt").arg("id", id))?;
        check_schema(v["schema_version"].as_u64().unwrap_or(u64::MAX) as u32, MANIFEST_SCHEMA)?;
        serde_json::from_value(v).map_err(|_| Event::new("snapshot.corrupt").arg("id", id))
    }
    pub fn latest_success(&self) -> Option<String> {
        std::fs::read_to_string(self.root.join("latest_success")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
    pub fn list_snapshots(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.root.join("snapshots")).into_iter().flatten().flatten()
            .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".manifest")).map(String::from)).collect();
        v.sort(); v
    }
    /// Vérification complète (hash de chaque objet).
    pub fn verify(&self, id: &str) -> Result<Vec<Event>, Event> {
        self.verify_with(id, VerifyMode::Full, &std::sync::atomic::AtomicBool::new(false))
    }
    /// Vérifie un point SANS rien copier. Quick : présence + taille. Full : relecture et hash. Annulable.
    pub fn verify_with(&self, id: &str, mode: VerifyMode, cancel: &std::sync::atomic::AtomicBool) -> Result<Vec<Event>, Event> {
        let m = self.load_manifest(id)?;
        let mut cache: std::collections::HashMap<String, Option<&'static str>> = Default::default();
        let mut problems = vec![];
        for e in &m.entries {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) { return Err(Event::new("verify.cancelled")); }
            let EntryKind::File { hash } = &e.kind else { continue };
            let bad = *cache.entry(hash.clone()).or_insert_with(|| {
                let p = self.object_path(hash);
                let Ok(md) = std::fs::metadata(&p) else { return Some("verify.object_missing") };
                if md.len() != e.size { return Some("verify.object_corrupt") }
                if mode == VerifyMode::Quick { return None }
                let Ok(f) = std::fs::File::open(&p) else { return Some("verify.object_missing") };
                let mut h = blake3::Hasher::new();
                if h.update_reader(f).is_err() || h.finalize().to_hex().as_str() != hash { Some("verify.object_corrupt") } else { None }
            });
            if let Some(code) = bad { problems.push(Event::new(code).arg("path", &e.path)); }
        }
        Ok(problems)
    }
}
