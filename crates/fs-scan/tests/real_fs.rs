//! Essais sur un vrai système de fichiers (répertoires temporaires isolés).
use fs_scan::{paths::*, scan::*};
use std::{fs, os::unix::fs::symlink, path::Path};
use tempfile::tempdir;

fn code(r: Result<(std::path::PathBuf, std::path::PathBuf), core_types::Event>) -> String { r.unwrap_err().code }

#[test] fn refuses_same_and_nested() {
    let t = tempdir().unwrap(); let s = t.path().join("src"); fs::create_dir(&s).unwrap();
    assert_eq!(code(validate_roots(&s, &s)), "path.same");
    assert_eq!(code(validate_roots(&s, &s.join("sub/dest"))), "path.dest_inside_source");
    assert_eq!(code(validate_roots(&s, t.path())), "path.source_inside_dest");
}
#[test] fn refuses_nesting_through_symlink() {
    let t = tempdir().unwrap(); let s = t.path().join("src"); fs::create_dir(&s).unwrap();
    symlink(&s, t.path().join("alias")).unwrap();
    assert_eq!(code(validate_roots(&s, &t.path().join("alias/out"))), "path.dest_inside_source");
}
#[test] fn refuses_dotdot_and_relative_and_missing() {
    let t = tempdir().unwrap(); let s = t.path().join("src"); fs::create_dir(&s).unwrap();
    assert_eq!(code(validate_roots(&s, &s.join("../x"))), "path.invalid");
    assert_eq!(code(validate_roots(Path::new("rel"), t.path())), "path.source_missing");
    assert_eq!(code(validate_roots(&t.path().join("nope"), &t.path().join("d"))), "path.source_missing");
}
#[test] fn accepts_distinct_with_nonexistent_dest() {
    let t = tempdir().unwrap(); let (s, d) = (t.path().join("src"), t.path().join("dst/new"));
    fs::create_dir(&s).unwrap();
    assert!(validate_roots(&s, &d).is_ok());
    assert!(!d.exists(), "ne doit rien créer");
}
#[test] fn safe_join_blocks_escape() {
    let r = Path::new("/r");
    assert!(safe_join(r, Path::new("../etc")).is_err());
    assert!(safe_join(r, Path::new("/etc")).is_err());
    assert!(safe_join(r, Path::new("a/b")).is_ok());
}
#[test] fn symlinks_recorded_not_followed_and_loop_safe() {
    let t = tempdir().unwrap(); let s = t.path();
    fs::create_dir(s.join("d")).unwrap(); fs::write(s.join("d/f.txt"), "x").unwrap();
    symlink(s, s.join("d/loop")).unwrap();          // boucle potentielle
    symlink("d/f.txt", s.join("lnk")).unwrap();
    let r = scan(s);
    assert!(r.entries.iter().any(|e| e.rel == Path::new("lnk") && matches!(e.kind, Kind::Symlink{..})));
    assert!(r.entries.iter().any(|e| e.rel == Path::new("d/loop") && matches!(e.kind, Kind::Symlink{..})));
    assert_eq!(r.entries.len(), 4); // d, d/f.txt, d/loop, lnk
}
#[test] fn unicode_nfc_nfd_and_case_collisions_detected() {
    let t = tempdir().unwrap(); let s = t.path();
    fs::write(s.join("caf\u{e9}.txt"), "a").unwrap();        // NFC
    fs::write(s.join("cafe\u{301}.txt"), "b").unwrap();      // NFD (Linux garde les deux)
    fs::write(s.join("Readme"), "c").unwrap(); fs::write(s.join("README"), "d").unwrap();
    fs::write(s.join("été ☂ 日本 .txt"), "e").unwrap();      // accents, espaces, Unicode : pas de collision
    let r = scan(s);
    assert_eq!(r.collisions.len(), 2);
    assert_eq!(r.entries.len(), 5);
}
#[test] fn scan_never_modifies_source() {
    let t = tempdir().unwrap(); let s = t.path();
    fs::create_dir(s.join("a")).unwrap(); fs::write(s.join("a/f"), "hello").unwrap();
    let snap = |p: &Path| { let mut v: Vec<_> = scan(p).entries.into_iter().map(|e| (e.rel, e.size, e.mtime_ns)).collect(); v.sort(); v };
    let before = snap(s); let _ = scan(s); let after = snap(s);
    assert_eq!(before, after);
    assert_eq!(fs::read_to_string(s.join("a/f")).unwrap(), "hello");
}
#[test] fn unreadable_dir_is_error_not_deletion() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempdir().unwrap(); let s = t.path();
    fs::create_dir(s.join("locked")).unwrap(); fs::write(s.join("locked/f"), "x").unwrap();
    fs::set_permissions(s.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
    let still_denied = fs::read_dir(s.join("locked")).is_err(); // faux si exécuté en root
    let r = scan(s);
    fs::set_permissions(s.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
    if still_denied {
        assert_eq!(r.errors.len(), 1); assert_eq!(r.errors[0].code, "scan.unreadable");
        assert!(r.entries.iter().any(|e| e.rel == Path::new("locked"))); // le dossier reste connu
    } else { eprintln!("SKIP: exécuté en root, permissions non testables"); }
}
