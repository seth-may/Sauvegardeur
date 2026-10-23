//! Parcours déterministe. Les liens symboliques sont enregistrés, jamais suivis.
//! Une erreur de lecture est consignée : elle ne signifie JAMAIS « fichier supprimé ».
use core_types::Event;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind { File, Dir, Symlink { target: PathBuf }, Other }

#[derive(Debug, Clone)]
pub struct Entry { pub rel: PathBuf, pub kind: Kind, pub size: u64, pub mtime_ns: i128 }

#[derive(Debug, Default)]
pub struct ScanResult {
    pub entries: Vec<Entry>,
    /// Chemins illisibles (code stable + chemin) : état « partiel », pas une suppression.
    pub errors: Vec<Event>,
    /// Groupes de chemins qui entrent en collision après normalisation Unicode NFC + minuscules.
    pub collisions: Vec<Vec<PathBuf>>,
}

/// Clé de comparaison : NFC puis minuscules (détecte NFC/NFD et casse).
pub fn collision_key(rel: &Path) -> String {
    rel.to_string_lossy().nfc().collect::<String>().to_lowercase()
}

pub fn scan(root: &Path) -> ScanResult {
    let mut res = ScanResult::default();
    walk(root, Path::new(""), &mut res);
    res.entries.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for e in &res.entries { groups.entry(collision_key(&e.rel)).or_default().push(e.rel.clone()); }
    res.collisions = groups.into_values().filter(|g| g.len() > 1).collect();
    res
}

fn walk(root: &Path, rel: &Path, res: &mut ScanResult) {
    let dir = root.join(rel);
    let rd = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) => { res.errors.push(Event::new("scan.unreadable").arg("path", rel.display()).arg("kind", e.kind())); return; }
    };
    for item in rd {
        let Ok(item) = item else { res.errors.push(Event::new("scan.unreadable").arg("path", rel.display())); continue };
        let rel_child = rel.join(item.file_name());
        let md = match std::fs::symlink_metadata(item.path()) {
            Ok(m) => m,
            Err(e) => { res.errors.push(Event::new("scan.unreadable").arg("path", rel_child.display()).arg("kind", e.kind())); continue; }
        };
        let ft = md.file_type();
        let mtime_ns = md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos() as i128).unwrap_or(0);
        let kind = if ft.is_symlink() {
            Kind::Symlink { target: std::fs::read_link(item.path()).unwrap_or_default() }
        } else if ft.is_dir() { Kind::Dir } else if ft.is_file() { Kind::File } else { Kind::Other };
        let is_dir = kind == Kind::Dir;
        res.entries.push(Entry { rel: rel_child.clone(), kind, size: md.len(), mtime_ns });
        if is_dir { walk(root, &rel_child, res); }
    }
}
