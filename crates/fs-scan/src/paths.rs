//! Résolution canonique et refus des couples source/destination dangereux (spec §7).
use core_types::Event;
use std::path::{Component, Path, PathBuf};

fn err(code: &str, p: &Path) -> Event { Event::new(code).arg("path", p.display()) }

/// Canonicalise `p` même si la fin n'existe pas encore : on résout le plus long
/// préfixe existant (liens inclus) puis on rajoute le reste, sans `..` ni `.`.
pub fn canonicalize_lenient(p: &Path) -> Result<PathBuf, Event> {
    if !p.is_absolute() { return Err(err("path.not_absolute", p)); }
    let mut existing = p.to_path_buf();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match std::fs::canonicalize(&existing) {
            Ok(c) => {
                let mut out = c;
                for part in rest.iter().rev() { out.push(part); }
                return Ok(out);
            }
            Err(_) => {
                let Some(name) = existing.file_name() else { return Err(err("path.invalid", p)) };
                if name == ".." || name == "." { return Err(err("path.invalid", p)); }
                rest.push(name.to_os_string());
                if !existing.pop() { return Err(err("path.invalid", p)); }
            }
        }
    }
}

/// Refuse composants `..`, chemins vides ou relatifs.
pub fn reject_traversal(p: &Path) -> Result<(), Event> {
    if p.as_os_str().is_empty() || p.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err(err("path.invalid", p));
    }
    Ok(())
}

/// Valide le couple (source, destination). Retourne les chemins canoniques.
/// La source doit exister et être un dossier ; la destination peut ne pas exister encore.
pub fn validate_roots(source: &Path, dest: &Path) -> Result<(PathBuf, PathBuf), Event> {
    reject_traversal(source)?;
    reject_traversal(dest)?;
    let s = std::fs::canonicalize(source).map_err(|_| err("path.source_missing", source))?;
    if !s.is_dir() { return Err(err("path.source_not_dir", source)); }
    let d = canonicalize_lenient(dest)?;
    if s == d { return Err(err("path.same", &s)); }
    if d.starts_with(&s) { return Err(err("path.dest_inside_source", &d)); }
    if s.starts_with(&d) { return Err(err("path.source_inside_dest", &s)); }
    Ok((s, d))
}

/// Un chemin relatif issu d'un manifeste ne doit jamais sortir de la racine.
pub fn safe_join(root: &Path, rel: &Path) -> Result<PathBuf, Event> {
    if rel.is_absolute() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(err("path.escape", rel));
    }
    Ok(root.join(rel))
}
