use std::{fs, os::unix::fs::{symlink, PermissionsExt}, path::{Path, PathBuf}};
use store::{backup::run_backup, restore::restore, Repo, SnapshotStatus};
use tempfile::tempdir;

/// Empreinte de l'arbre : chemin, type, contenu, mode (pour comparer source/restauration).
fn tree(root: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs_scan::scan::scan(root).entries.iter().map(|e| {
        let p = root.join(&e.rel); let md = fs::symlink_metadata(&p).unwrap();
        let c = if md.is_file() { blake3::hash(&fs::read(&p).unwrap()).to_hex().to_string() } else if md.is_symlink() { fs::read_link(&p).unwrap().display().to_string() } else { "dir".into() };
        format!("{}|{}|{:o}", e.rel.display(), c, md.permissions().mode() & 0o7777)
    }).collect(); v.sort(); v
}
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let t = tempdir().unwrap(); let s = t.path().join("src"); let r = t.path().join("repo");
    fs::create_dir_all(s.join("sub/été ☂")).unwrap();
    fs::write(s.join("a.txt"), "alpha").unwrap(); fs::write(s.join("sub/b.bin"), vec![7u8; 3_000_000]).unwrap();
    fs::write(s.join("sub/été ☂/é.txt"), "accents").unwrap(); fs::write(s.join("dup.txt"), "alpha").unwrap();
    fs::set_permissions(s.join("a.txt"), fs::Permissions::from_mode(0o640)).unwrap();
    symlink("a.txt", s.join("lnk")).unwrap();
    (t, s, r)
}

#[test] fn full_backup_restore_identical_and_source_untouched() {
    let (t, s, r) = setup(); let before = tree(&s);
    let rep = run_backup(&s, &r).unwrap();
    assert_eq!(rep.status, SnapshotStatus::Succeeded); assert_eq!(tree(&s), before, "source modifiée !");
    let repo = Repo::open_or_init(&r).unwrap(); assert_eq!(repo.latest_success(), Some(rep.id.clone()));
    assert!(repo.verify(&rep.id).unwrap().is_empty());
    let out = t.path().join("out"); restore(&repo, &rep.id, None, &out).unwrap();
    assert_eq!(tree(&out), before);
}
#[test] fn dedup_identical_content_one_object() {
    let (_t, s, r) = setup(); let rep = run_backup(&s, &r).unwrap();
    assert_eq!(rep.reused, 1); // dup.txt == a.txt
}
#[test] fn incremental_old_versions_immutable_and_deletions_kept() {
    let (t, s, r) = setup(); let first = run_backup(&s, &r).unwrap(); let tree1 = tree(&s);
    fs::write(s.join("a.txt"), "BETA").unwrap(); fs::rename(s.join("dup.txt"), s.join("renamed.txt")).unwrap(); fs::remove_file(s.join("sub/b.bin")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = run_backup(&s, &r).unwrap();
    assert_eq!(second.copied, 1, "seul a.txt est nouveau"); assert_ne!(first.id, second.id);
    let repo = Repo::open_or_init(&r).unwrap();
    let (o1, o2) = (t.path().join("o1"), t.path().join("o2"));
    restore(&repo, &first.id, None, &o1).unwrap(); restore(&repo, &second.id, None, &o2).unwrap();
    // les mtime de a.txt diffèrent : on compare contenus/chemins
    let strip = |v: Vec<String>| v.into_iter().map(|l| l.split('|').take(2).collect::<Vec<_>>().join("|")).collect::<Vec<_>>();
    assert_eq!(strip(tree(&o1)), strip(tree1), "ancienne version intacte");
    assert!(!o2.join("sub/b.bin").exists() && o1.join("sub/b.bin").exists());
    assert_eq!(fs::read_to_string(o2.join("a.txt")).unwrap(), "BETA");
}
#[test] fn partial_does_not_replace_latest_success() {
    let (_t, s, r) = setup(); let ok = run_backup(&s, &r).unwrap();
    fs::write(s.join("secret"), "x").unwrap(); fs::set_permissions(s.join("secret"), fs::Permissions::from_mode(0)).unwrap();
    if fs::read(s.join("secret")).is_ok() { eprintln!("SKIP root"); return; }
    let p = run_backup(&s, &r).unwrap();
    assert_eq!(p.status, SnapshotStatus::Partial); assert!(!p.errors.is_empty());
    assert_eq!(Repo::open_or_init(&r).unwrap().latest_success(), Some(ok.id));
    fs::set_permissions(s.join("secret"), fs::Permissions::from_mode(0o600)).unwrap();
}
#[test] fn restore_refuses_nonempty_target_and_traversal() {
    let (t, s, r) = setup(); let rep = run_backup(&s, &r).unwrap(); let repo = Repo::open_or_init(&r).unwrap();
    let out = t.path().join("full"); fs::create_dir(&out).unwrap(); fs::write(out.join("x"), "1").unwrap();
    assert_eq!(restore(&repo, &rep.id, None, &out).unwrap_err().code, "restore.target_not_empty");
    // manifeste falsifié avec "../evil"
    let mp = repo.manifest_path(&rep.id); let txt = fs::read_to_string(&mp).unwrap().replace("\"a.txt\"", "\"../evil\"");
    fs::write(&mp, txt).unwrap();
    assert_eq!(restore(&repo, &rep.id, None, &t.path().join("o")).unwrap_err().code, "path.escape");
    assert!(!t.path().join("evil").exists());
}
#[test] fn partial_restore_of_file_and_folder() {
    let (t, s, r) = setup(); let rep = run_backup(&s, &r).unwrap(); let repo = Repo::open_or_init(&r).unwrap();
    let o = t.path().join("p"); restore(&repo, &rep.id, Some("sub"), &o).unwrap();
    assert_eq!(fs::read_to_string(o.join("sub/été ☂/é.txt")).unwrap(), "accents"); assert!(!o.join("a.txt").exists());
}
#[test] fn verify_detects_corruption() {
    let (_t, s, r) = setup(); let rep = run_backup(&s, &r).unwrap(); let repo = Repo::open_or_init(&r).unwrap();
    let m = repo.load_manifest(&rep.id).unwrap();
    let h = match &m.entries.iter().find(|e| e.path == "a.txt").unwrap().kind { store::EntryKind::File { hash } => hash.clone(), _ => panic!() };
    let p = repo.object_path(&h); fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap(); fs::write(&p, "corrompu").unwrap();
    assert_eq!(repo.verify(&rep.id).unwrap().len(), 2); // a.txt et dup.txt partagent l'objet
}
#[test] fn future_formats_refused_and_nesting_refused() {
    let (t, s, r) = setup(); let rep = run_backup(&s, &r).unwrap();
    let repo = Repo::open_or_init(&r).unwrap(); let mp = repo.manifest_path(&rep.id);
    fs::write(&mp, fs::read_to_string(&mp).unwrap().replace("\"schema_version\": 1", "\"schema_version\": 99")).unwrap();
    assert_eq!(repo.load_manifest(&rep.id).unwrap_err().code, "schema.unknown_future");
    fs::write(r.join("repo.json"), r#"{"format_version":99,"repo_id":"x"}"#).unwrap();
    assert_eq!(Repo::open_or_init(&r).err().unwrap().code, "schema.unknown_future");
    assert_eq!(run_backup(&s, &s.join("in")).unwrap_err().code, "path.dest_inside_source");
    let _ = t;
}
