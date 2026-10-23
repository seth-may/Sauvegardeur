use retention::{execute, plan};
use std::{fs, path::{Path, PathBuf}, sync::atomic::{AtomicBool, Ordering}, time::Duration};
use store::{backup::{run_backup, run_backup_with}, restore::restore, Repo, SnapshotStatus, VerifyMode};
use tempfile::tempdir;

/// n sauvegardes ; chaque version change `v.txt` (contenu unique) ; `common.bin` est partagé.
fn history(n: usize) -> (tempfile::TempDir, PathBuf, PathBuf, Vec<String>) {
    let t = tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); fs::create_dir(&s).unwrap();
    fs::write(s.join("common.bin"), vec![1u8; 100_000]).unwrap();
    let mut ids = vec![];
    for i in 0..n {
        fs::write(s.join("v.txt"), format!("version-{i}-{}", "x".repeat(1000 + i))).unwrap();
        ids.push(run_backup(&s, &r).unwrap().id);
        std::thread::sleep(Duration::from_millis(1100)); // ids ordonnés par seconde de création
    }
    (t, s, r, ids)
}
fn snapshot_of_repo(r: &Path) -> Vec<String> { // empreinte du dépôt : chemins + tailles
    let mut v = vec![]; let mut st = vec![r.to_path_buf()];
    while let Some(d) = st.pop() { for e in fs::read_dir(d).unwrap().flatten() { let p = e.path(); if p.is_dir() { st.push(p) } else { v.push(format!("{}:{}", p.display(), p.metadata().unwrap().len())) } } }
    v.sort(); v
}

#[test] fn plan_keeps_newest_and_counts_only_unique_objects() {
    let (_t, _s, r, ids) = history(4); let repo = Repo::open_or_init(&r).unwrap();
    let p = plan(&repo, 2).unwrap();
    assert_eq!(p.remove.iter().map(|x| x.id.clone()).collect::<Vec<_>>(), ids[..2].to_vec());
    assert_eq!(p.keep, ids[2..].to_vec());
    assert_eq!(p.reclaim_objects, 2, "common.bin est partagé : non récupérable");
    assert_eq!(p.reclaim_bytes, 1010 + 1011, "v.txt des versions 0 et 1 : 10+1000 et 10+1001 octets");
}
#[test] fn invalid_policy_and_large_keep() {
    let (_t, _s, r, _) = history(2); let repo = Repo::open_or_init(&r).unwrap();
    assert_eq!(plan(&repo, 0).unwrap_err().code, "retention.invalid_policy");
    let p = plan(&repo, 10).unwrap(); assert!(p.remove.is_empty() && p.reclaim_objects == 0);
}
#[test] fn refused_without_user_or_confirmation_deletes_nothing() {
    let (_t, _s, r, _) = history(3); let repo = Repo::open_or_init(&r).unwrap(); let p = plan(&repo, 1).unwrap();
    let before = snapshot_of_repo(&r);
    assert_eq!(execute(&repo, 1, &p.plan_id, false).unwrap_err().code, "purge.deferred");
    assert_eq!(execute(&repo, 1, "mauvais", true).unwrap_err().code, "purge.plan_stale");
    assert_eq!(snapshot_of_repo(&r), before);
}
#[test] fn execute_removes_old_keeps_new_restorable_and_shared_objects() {
    let (t, s, r, ids) = history(4); let repo = Repo::open_or_init(&r).unwrap(); let p = plan(&repo, 2).unwrap();
    let rep = execute(&repo, 2, &p.plan_id, true).unwrap();
    assert_eq!((rep.removed_snapshots, rep.deleted_objects), (2, 2)); assert_eq!(rep.freed_bytes, p.reclaim_bytes);
    assert_eq!(repo.list_snapshots(), ids[2..].to_vec()); assert_eq!(repo.latest_success().as_ref(), ids.last());
    for id in &ids[2..] { assert!(repo.verify_with(id, VerifyMode::Full, &AtomicBool::new(false)).unwrap().is_empty()); }
    let out = t.path().join("o"); restore(&repo, ids.last().unwrap(), None, &out).unwrap();
    assert_eq!(fs::read(out.join("v.txt")).unwrap(), fs::read(s.join("v.txt")).unwrap());
    assert_eq!(fs::read(out.join("common.bin")).unwrap().len(), 100_000);
    assert!(fs::read_dir(r.join("snapshots")).unwrap().flatten().all(|e| e.path().extension().unwrap() == "manifest"));
}
#[test] fn last_success_never_removed_even_with_keep_1() {
    let (_t, _s, r, ids) = history(3); let repo = Repo::open_or_init(&r).unwrap();
    let p = plan(&repo, 1).unwrap(); assert_eq!(p.keep, vec![ids[2].clone()]);
    execute(&repo, 1, &p.plan_id, true).unwrap(); assert_eq!(repo.list_snapshots(), vec![ids[2].clone()]);
}
#[test] fn plan_goes_stale_when_repo_changes() {
    let (_t, s, r, _) = history(3); let repo = Repo::open_or_init(&r).unwrap(); let p = plan(&repo, 1).unwrap();
    fs::write(s.join("v.txt"), "autre").unwrap(); run_backup(&s, &r).unwrap();
    assert_eq!(execute(&repo, 1, &p.plan_id, true).unwrap_err().code, "purge.plan_stale");
}
#[test] fn old_interrupted_removed_new_one_kept() {
    let t = tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); fs::create_dir(&s).unwrap();
    for i in 0..30 { fs::write(s.join(format!("f{i}")), format!("a{i}")).unwrap(); }
    let ok1 = run_backup(&s, &r).unwrap().id; std::thread::sleep(Duration::from_millis(1100));
    let c = AtomicBool::new(false); fs::write(s.join("zz"), "only-in-interrupted").unwrap();
    let int1 = run_backup_with(&s, &r, &c, &mut |p| if p.processed >= 3 { c.store(true, Ordering::Relaxed) }).unwrap(); assert_eq!(int1.status, SnapshotStatus::Interrupted);
    std::thread::sleep(Duration::from_millis(1100)); let ok2 = run_backup(&s, &r).unwrap().id; std::thread::sleep(Duration::from_millis(1100));
    let c2 = AtomicBool::new(false); let int2 = run_backup_with(&s, &r, &c2, &mut |p| if p.processed >= 3 { c2.store(true, Ordering::Relaxed) }).unwrap();
    let repo = Repo::open_or_init(&r).unwrap(); let p = plan(&repo, 5).unwrap();
    assert_eq!(p.remove.iter().map(|x| x.id.clone()).collect::<Vec<_>>(), vec![int1.id.clone()], "ancien interrompu retiré seulement");
    assert!(p.keep.contains(&int2.id) && p.keep.contains(&ok1) && p.keep.contains(&ok2));
    execute(&repo, 5, &p.plan_id, true).unwrap(); assert_eq!(repo.latest_success(), Some(ok2));
}
#[test] fn unreadable_manifest_blocks_everything() {
    let (_t, _s, r, ids) = history(3); let repo = Repo::open_or_init(&r).unwrap(); let before = snapshot_of_repo(&r);
    fs::write(repo.manifest_path(&ids[0]), "{pas du json").unwrap(); let corrupt = snapshot_of_repo(&r);
    assert_eq!(plan(&repo, 1).unwrap_err().code, "purge.unreadable_manifest");
    assert_eq!(execute(&repo, 1, "x", true).unwrap_err().code, "purge.unreadable_manifest");
    assert_eq!(snapshot_of_repo(&r), corrupt); let _ = before;
}
#[test] fn orphans_and_interrupted_purge_are_swept() {
    let (_t, _s, r, ids) = history(3); let repo = Repo::open_or_init(&r).unwrap();
    fs::create_dir_all(r.join("objects/ab")).unwrap(); fs::write(r.join(format!("objects/ab/{}", "c".repeat(62))), "orphelin").unwrap();
    // purge interrompue : un manifeste déjà mis de côté
    fs::rename(repo.manifest_path(&ids[0]), r.join(format!("snapshots/{}.manifest.purging", ids[0]))).unwrap();
    let p = plan(&repo, 5).unwrap(); assert!(p.reclaim_objects >= 2, "orphelin + objet du point mis de côté");
    let rep = execute(&repo, 5, &p.plan_id, true).unwrap(); assert!(rep.deleted_objects >= 2);
    assert!(!r.join(format!("snapshots/{}.manifest.purging", ids[0])).exists());
    for id in &ids[1..] { assert!(repo.verify(id).unwrap().is_empty()); }
}
#[test] fn purge_refused_while_repo_locked() {
    let (_t, _s, r, _) = history(2); let repo = Repo::open_or_init(&r).unwrap(); let p = plan(&repo, 1).unwrap();
    let _g = repo.lock().unwrap(); assert_eq!(execute(&repo, 1, &p.plan_id, true).unwrap_err().code, "repo.locked");
}
