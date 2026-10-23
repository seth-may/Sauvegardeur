//! Sauvegarde complète/incrémentale (les objets déjà présents sont réutilisés sans recopie).
//! La source n'est qu'ouverte en lecture. Un point partiel ne remplace pas `latest_success`.
use crate::*;
use fs_scan::scan::{scan, Kind};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct Report { pub id: String, pub status: SnapshotStatus, pub scanned: u64, pub copied: u64, pub reused: u64, pub bytes: u64, pub errors: Vec<Event> }

fn sig(p: &Path) -> std::io::Result<(u64, i128)> {
    let m = std::fs::metadata(p)?;
    Ok((m.len(), m.mtime() as i128 * 1_000_000_000 + m.mtime_nsec() as i128))
}

/// Copie `src` vers un fichier temporaire en calculant le hash ; détecte un changement pendant la lecture.
fn ingest(repo: &Repo, src: &Path) -> Result<(String, u64, bool), Event> {
    let before = sig(src).map_err(|e| io_err("backup.read_failed", src, &e))?;
    let mut f = std::fs::File::open(src).map_err(|e| io_err("backup.read_failed", src, &e))?;
    let tmp = repo.root.join("tmp").join(format!("{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
    let mut out = std::fs::File::create(&tmp).map_err(|e| io_err("backup.write_failed", &tmp, &e))?;
    let mut h = blake3::Hasher::new(); let mut buf = vec![0u8; 1 << 20]; let mut n_total = 0u64;
    let r: std::io::Result<()> = (|| loop {
        let n = f.read(&mut buf)?; if n == 0 { return Ok(()); }
        h.update(&buf[..n]); out.write_all(&buf[..n])?; n_total += n as u64;
    })();
    if let Err(e) = r { let _ = std::fs::remove_file(&tmp); return Err(io_err("backup.read_failed", src, &e)); }
    let _ = out.sync_all();
    if sig(src).ok() != Some(before) || n_total != before.0 { let _ = std::fs::remove_file(&tmp); return Err(Event::new("backup.file_changed").arg("path", src.display())); }
    let hash = h.finalize().to_hex().to_string();
    let dst = repo.object_path(&hash);
    if dst.exists() { let _ = std::fs::remove_file(&tmp); return Ok((hash, n_total, false)); }
    std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| io_err("backup.write_failed", &dst, &e))?;
    std::fs::rename(&tmp, &dst).map_err(|e| io_err("backup.write_failed", &dst, &e))?;
    let _ = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o444)); // objets immuables
    Ok((hash, n_total, true))
}

pub fn run_backup(source: &Path, repo_root: &Path) -> Result<Report, Event> {
    let (src, dst) = fs_scan::paths::validate_roots(source, repo_root)?;
    let repo = Repo::open_or_init(&dst)?;
    let sc = scan(&src);
    let mut errors = sc.errors.clone();
    let warnings: Vec<Event> = sc.collisions.iter().map(|g| Event::new("scan.name_collision").arg("paths", g.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" | "))).collect();
    let (mut entries, mut copied, mut reused, mut bytes) = (vec![], 0u64, 0u64, 0u64);
    for e in &sc.entries {
        let Some(rel) = e.rel.to_str() else { errors.push(Event::new("path.non_utf8").arg("path", e.rel.display())); continue };
        let full = src.join(&e.rel);
        let mode = std::fs::symlink_metadata(&full).map(|m| m.mode() & 0o7777).unwrap_or(0);
        let kind = match &e.kind {
            Kind::Dir => EntryKind::Dir,
            Kind::Symlink { target } => EntryKind::Symlink { target: target.to_string_lossy().into_owned() },
            Kind::Other => { errors.push(Event::new("scan.unsupported_type").arg("path", rel)); continue }
            Kind::File => {
                let mut res = ingest(&repo, &full);
                if matches!(&res, Err(ev) if ev.code == "backup.file_changed") { res = ingest(&repo, &full); } // une relecture
                match res {
                    Ok((hash, n, new)) => { if new { copied += 1; bytes += n } else { reused += 1 } EntryKind::File { hash } }
                    Err(ev) => { errors.push(ev); continue }
                }
            }
        };
        entries.push(ManifestEntry { path: rel.into(), kind, size: e.size, mtime_ns: e.mtime_ns, mode });
    }
    let created = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let status = if errors.is_empty() { SnapshotStatus::Succeeded } else { SnapshotStatus::Partial };
    let tag = blake3::hash(format!("{created}{}{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)).as_bytes()).to_hex().to_string();
    let id = format!("{created:012}-{}", &tag[..8]);
    let m = Manifest { schema_version: MANIFEST_SCHEMA, id: id.clone(), created_unix: created, status, entries, errors: errors.clone(), warnings };
    // Le manifeste n'apparaît qu'une fois complet (rename atomique) ; puis seulement le pointeur de réussite.
    atomic_write(&repo.manifest_path(&id), &serde_json::to_vec_pretty(&m).unwrap()).map_err(|e| io_err("backup.manifest_failed", &repo.root, &e))?;
    if status == SnapshotStatus::Succeeded {
        atomic_write(&repo.root.join("latest_success"), id.as_bytes()).map_err(|e| io_err("backup.manifest_failed", &repo.root, &e))?;
    }
    Ok(Report { id, status, scanned: sc.entries.len() as u64, copied, reused, bytes, errors })
}
