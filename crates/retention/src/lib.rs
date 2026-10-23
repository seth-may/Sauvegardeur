//! Rétention et purge (spec §7). Principes :
//! - le plan est un calcul pur, sans rien supprimer ;
//! - l'exécution exige : utilisateur présent, identifiant de plan confirmé, verrou exclusif, plan inchangé ;
//! - jamais la dernière réussite, jamais sous `keep` réussites (>= 1), jamais un objet encore référencé ;
//! - un manifeste illisible bloque tout : on ne peut pas prouver qu'un objet est inutilisé.
use core_types::Event;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use store::{EntryKind, Manifest, Repo, SnapshotStatus};

#[derive(Debug, Clone, Serialize)]
pub struct SnapInfo { pub id: String, pub created_unix: u64, pub status: SnapshotStatus }

#[derive(Debug, Clone, Serialize)]
pub struct PurgePlan {
    /// Empreinte à confirmer : change dès que les versions ou les objets concernés changent.
    pub plan_id: String,
    pub remove: Vec<SnapInfo>,
    pub keep: Vec<String>,
    /// Objets qu'aucune version conservée ne référence (dont d'éventuels orphelins d'un arrêt brutal).
    pub reclaim_objects: u64,
    pub reclaim_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct PurgeReport { pub removed_snapshots: u64, pub deleted_objects: u64, pub freed_bytes: u64 }

fn key(m: &Manifest) -> (u64, String) { (m.created_unix, m.id.clone()) }

fn load_all(repo: &Repo) -> Result<Vec<Manifest>, Event> {
    let mut v = vec![];
    for id in repo.list_snapshots() {
        v.push(repo.load_manifest(&id).map_err(|e| Event::new("purge.unreadable_manifest").arg("id", &id).arg("cause", e.code))?);
    }
    v.sort_by_key(key);
    Ok(v)
}

fn hashes(m: &Manifest) -> impl Iterator<Item = &String> {
    m.entries.iter().filter_map(|e| if let EntryKind::File { hash } = &e.kind { Some(hash) } else { None })
}

fn all_objects(repo: &Repo) -> Vec<(String, PathBuf, u64)> {
    let mut v = vec![];
    for d in std::fs::read_dir(repo.root.join("objects")).into_iter().flatten().flatten() {
        let prefix = d.file_name().to_string_lossy().into_owned();
        for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
            let size = f.metadata().map(|m| m.len()).unwrap_or(0);
            v.push((format!("{prefix}{}", f.file_name().to_string_lossy()), f.path(), size));
        }
    }
    v
}

/// Calcul du plan. `keep` = nombre de sauvegardes RÉUSSIES à conserver (minimum 1).
pub fn plan(repo: &Repo, keep: usize) -> Result<PurgePlan, Event> {
    if keep == 0 { return Err(Event::new("retention.invalid_policy").arg("keep", keep)); }
    let all = load_all(repo)?;
    let succ: Vec<&Manifest> = all.iter().filter(|m| m.status == SnapshotStatus::Succeeded).collect();
    let mut keep_ids: BTreeSet<String> = succ.iter().rev().take(keep).map(|m| m.id.clone()).collect();
    if let Some(l) = repo.latest_success() { keep_ids.insert(l); }          // la dernière réussite est intouchable
    // Points partiels/interrompus : retirables seulement s'ils sont plus anciens que la dernière réussite.
    let newest_ok = succ.last().map(|m| key(m));
    for m in &all {
        if m.status != SnapshotStatus::Succeeded && newest_ok.as_ref().is_none_or(|n| &key(m) > n) { keep_ids.insert(m.id.clone()); }
    }
    if succ.is_empty() { keep_ids.extend(all.iter().map(|m| m.id.clone())); } // rien à purger sans réussite
    let remove: Vec<SnapInfo> = all.iter().filter(|m| !keep_ids.contains(&m.id))
        .map(|m| SnapInfo { id: m.id.clone(), created_unix: m.created_unix, status: m.status }).collect();
    let live: HashSet<&String> = all.iter().filter(|m| keep_ids.contains(&m.id)).flat_map(hashes).collect();
    let mut dead: Vec<(String, u64)> = all_objects(repo).into_iter().filter(|(h, _, _)| !live.contains(h)).map(|(h, _, s)| (h, s)).collect();
    dead.sort();
    let mut h = blake3::Hasher::new(); h.update(b"plan-v1");
    for r in &remove { h.update(r.id.as_bytes()); h.update(b"|"); }
    for k in &keep_ids { h.update(k.as_bytes()); h.update(b"|"); }
    for (o, _) in &dead { h.update(o.as_bytes()); h.update(b"|"); }
    Ok(PurgePlan {
        plan_id: h.finalize().to_hex()[..16].to_string(), remove, keep: keep_ids.into_iter().collect(),
        reclaim_objects: dead.len() as u64, reclaim_bytes: dead.iter().map(|(_, s)| *s).sum(),
    })
}

/// Exécute un plan confirmé. Sans utilisateur présent (tâche automatique) : différé, rien n'est supprimé.
pub fn execute(repo: &Repo, keep: usize, confirmed_plan_id: &str, user_present: bool) -> Result<PurgeReport, Event> {
    if !user_present { return Err(Event::new("purge.deferred")); }
    let _lock = repo.lock()?;
    let p = plan(repo, keep)?;
    if p.plan_id != confirmed_plan_id { return Err(Event::new("purge.plan_stale")); }
    // 1) retirer les manifestes de la vue (rename atomique) : ces points disparaissent d'un coup
    let snaps = repo.root.join("snapshots");
    for r in &p.remove {
        let from = repo.manifest_path(&r.id);
        std::fs::rename(&from, snaps.join(format!("{}.manifest.purging", r.id))).map_err(|e| Event::new("purge.delete_failed").arg("path", from.display()).arg("kind", e.kind()))?;
    }
    // 2) supprimer les objets non référencés par les manifestes restants
    let live_m = load_all(repo)?;
    let live: HashSet<&String> = live_m.iter().flat_map(hashes).collect();
    let (mut n, mut bytes) = (0u64, 0u64);
    for (h, path, size) in all_objects(repo) {
        if live.contains(&h) { continue }
        std::fs::remove_file(&path).map_err(|e| Event::new("purge.delete_failed").arg("path", path.display()).arg("kind", e.kind()))?;
        let _ = std::fs::remove_dir(path.parent().unwrap()); // seulement si vide
        n += 1; bytes += size;
    }
    // 3) effacer les manifestes retirés (aussi ceux laissés par une purge interrompue)
    for e in std::fs::read_dir(&snaps).into_iter().flatten().flatten() {
        if e.path().extension().is_some_and(|x| x == "purging") { let _ = std::fs::remove_file(e.path()); }
    }
    // 4) garde-fou final : la dernière réussite doit rester lisible
    if let Some(l) = repo.latest_success() { repo.load_manifest(&l)?; }
    Ok(PurgeReport { removed_snapshots: p.remove.len() as u64, deleted_objects: n, freed_bytes: bytes })
}
