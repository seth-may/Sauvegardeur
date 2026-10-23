//! Restauration vers un NOUVEL emplacement (jamais d'écrasement en M2).
use crate::*;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, UNIX_EPOCH};

/// Restaure le point `id` (ou le seul `subpath` : fichier ou dossier) dans `target`.
/// `target` doit être absent ou vide. Chaque fichier est écrit en temporaire, vérifié par hash, puis renommé.
pub fn restore(repo: &Repo, id: &str, subpath: Option<&str>, target: &Path) -> Result<u64, Event> {
    let m = repo.load_manifest(id)?;
    if target.exists() && std::fs::read_dir(target).map(|mut r| r.next().is_some()).unwrap_or(true) {
        return Err(Event::new("restore.target_not_empty").arg("path", target.display()));
    }
    std::fs::create_dir_all(target).map_err(|e| io_err("restore.write_failed", target, &e))?;
    let sel = |p: &str| match subpath { None => true, Some(s) => p == s || p.starts_with(&format!("{s}/")) };
    let mut n = 0u64;
    let mut dirs: Vec<(PathBuf, &ManifestEntry)> = vec![];
    for e in m.entries.iter().filter(|e| sel(&e.path)) {
        let dest = fs_scan::paths::safe_join(target, Path::new(&e.path))?; // refuse toute sortie de `target`
        if let Some(parent) = dest.parent() { std::fs::create_dir_all(parent).map_err(|x| io_err("restore.write_failed", parent, &x))?; }
        if dest.symlink_metadata().is_ok() && !matches!(e.kind, EntryKind::Dir) { return Err(Event::new("restore.collision").arg("path", &e.path)); }
        match &e.kind {
            EntryKind::Dir => { std::fs::create_dir_all(&dest).map_err(|x| io_err("restore.write_failed", &dest, &x))?; dirs.push((dest, e)); }
            EntryKind::Symlink { target: t } => std::os::unix::fs::symlink(t, &dest).map_err(|x| io_err("restore.write_failed", &dest, &x))?,
            EntryKind::File { hash } => {
                let mut src = std::fs::File::open(repo.object_path(hash)).map_err(|_| Event::new("restore.object_missing").arg("path", &e.path))?;
                let tmp = dest.with_extension("sauvegardeur-tmp");
                let mut out = std::fs::File::create(&tmp).map_err(|x| io_err("restore.write_failed", &tmp, &x))?;
                let (mut h, mut buf) = (blake3::Hasher::new(), vec![0u8; 1 << 20]);
                loop {
                    let k = src.read(&mut buf).map_err(|x| io_err("restore.read_failed", &dest, &x))?; if k == 0 { break }
                    h.update(&buf[..k]); out.write_all(&buf[..k]).map_err(|x| io_err("restore.write_failed", &tmp, &x))?;
                }
                if h.finalize().to_hex().as_str() != hash { let _ = std::fs::remove_file(&tmp); return Err(Event::new("restore.object_corrupt").arg("path", &e.path)); }
                let _ = out.set_modified(UNIX_EPOCH + Duration::from_nanos(e.mtime_ns.max(0) as u64));
                out.sync_all().map_err(|x| io_err("restore.write_failed", &tmp, &x))?; drop(out);
                let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(e.mode));
                std::fs::rename(&tmp, &dest).map_err(|x| io_err("restore.write_failed", &dest, &x))?;
            }
        }
        n += 1;
    }
    for (d, e) in dirs.into_iter().rev() { let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(e.mode)); }
    Ok(n)
}
