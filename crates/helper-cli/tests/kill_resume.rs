//! Essais de bout en bout sur le vrai binaire : arrêt brutal (SIGKILL), annulation, reprise.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::{fs, path::Path};

fn spawn() -> Child { Command::new(env!("CARGO_BIN_EXE_helper-cli")).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap() }
fn send(c: &mut Child, l: &str) { let i = c.stdin.as_mut().unwrap(); writeln!(i, "{l}").unwrap(); i.flush().unwrap(); }
fn make_source(s: &Path) {
    fs::create_dir_all(s).unwrap();
    for i in 0..60u32 { // 60 fichiers de 4 Mo, tous distincts
        let mut v = vec![0u8; 4 << 20]; for (k, b) in v.iter_mut().enumerate() { *b = (k as u32).wrapping_mul(2654435761).wrapping_add(i * 7919) as u8 }
        fs::write(s.join(format!("f{i:02}.bin")), v).unwrap();
    }
}
fn same_tree(a: &Path, b: &Path) {
    let (sa, sb) = (fs_scan::scan::scan(a), fs_scan::scan::scan(b));
    assert_eq!(sa.entries.len(), sb.entries.len());
    for e in sa.entries.iter().filter(|e| e.kind == fs_scan::scan::Kind::File) { assert_eq!(fs::read(a.join(&e.rel)).unwrap(), fs::read(b.join(&e.rel)).unwrap()); }
}
fn wait_type(r: &mut BufReader<std::process::ChildStdout>, t: &str) -> String {
    loop { let mut l = String::new(); assert!(r.read_line(&mut l).unwrap() > 0, "flux fermé avant {t}"); if l.contains(&format!("\"type\":\"{t}\"")) { return l } }
}
fn backup_cmd(s: &Path, r: &Path) -> String { format!(r#"{{"cmd":"backup","source":"{}","repo":"{}"}}"#, s.display(), r.display()) }

#[test] fn sigkill_midcopy_then_resume_restores_identically() {
    let t = tempfile::tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); make_source(&s);
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    send(&mut c, &backup_cmd(&s, &r)); wait_type(&mut rd, "progress");
    std::thread::sleep(std::time::Duration::from_millis(30)); c.kill().unwrap(); c.wait().unwrap(); // arrêt brutal
    let repo = store::Repo::open_or_init(&r).unwrap();
    assert!(repo.latest_success().is_none() && repo.list_snapshots().is_empty(), "aucun point ne doit exister");
    let mut c2 = spawn(); let mut rd2 = BufReader::new(c2.stdout.take().unwrap());
    send(&mut c2, &backup_cmd(&s, &r)); let done = wait_type(&mut rd2, "done");
    assert!(done.contains("\"status\":\"succeeded\""), "{done}");
    let id = repo.latest_success().unwrap(); assert!(repo.verify(&id).unwrap().is_empty());
    let out = t.path().join("out"); store::restore::restore(&repo, &id, None, &out).unwrap(); same_tree(&s, &out);
    drop(c2.stdin.take()); c2.wait().unwrap();
}
#[test] fn cancel_command_yields_interrupted_and_keeps_last_success() {
    let t = tempfile::tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); make_source(&s);
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    send(&mut c, &backup_cmd(&s, &r)); wait_type(&mut rd, "progress"); send(&mut c, r#"{"cmd":"cancel"}"#);
    let done = wait_type(&mut rd, "done"); assert!(done.contains("\"status\":\"interrupted\""), "{done}");
    assert!(store::Repo::open_or_init(&r).unwrap().latest_success().is_none());
    drop(c.stdin.take()); c.wait().unwrap();
}
#[test] fn second_backup_while_busy_is_refused() {
    let t = tempfile::tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); make_source(&s);
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    send(&mut c, &backup_cmd(&s, &r)); wait_type(&mut rd, "progress"); send(&mut c, &backup_cmd(&s, &r));
    wait_type(&mut rd, "failed"); send(&mut c, r#"{"cmd":"cancel"}"#); wait_type(&mut rd, "done");
    drop(c.stdin.take()); c.wait().unwrap();
}
#[test] fn protocol_basics() {
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    send(&mut c, r#"{"cmd":"hello"}"#); assert!(wait_type(&mut rd, "hello").contains("\"protocol\":1"));
    send(&mut c, "zzz"); wait_type(&mut rd, "error"); drop(c.stdin.take()); c.wait().unwrap();
}

fn ask(c: &mut Child, rd: &mut BufReader<std::process::ChildStdout>, cmd: &str, t: &str) -> String { send(c, cmd); wait_type(rd, t) }

#[test] fn verify_restore_list_via_protocol() {
    let t = tempfile::tempdir().unwrap(); let (s, r, o) = (t.path().join("src"), t.path().join("repo"), t.path().join("out"));
    fs::create_dir_all(&s).unwrap(); fs::write(s.join("a.txt"), "alpha").unwrap(); fs::write(s.join("é ☂.txt"), "beta").unwrap();
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    assert!(ask(&mut c, &mut rd, &backup_cmd(&s, &r), "done").contains("succeeded"));
    let l = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"list","repo":"{}"}}"#, r.display()), "snapshots"); assert!(l.contains("latest_success\":\""), "{l}");
    for full in ["false", "true"] {
        let v = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"verify","repo":"{}","full":{full}}}"#, r.display()), "verified"); assert!(v.contains("\"problems\":[]"), "{v}");
    }
    let id = serde_json::from_str::<serde_json::Value>(&l).unwrap()["latest_success"].as_str().unwrap().to_string();
    let rs = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"restore","repo":"{}","id":"{id}","target":"{}"}}"#, r.display(), o.display()), "restored"); assert!(rs.contains("\"count\":2"), "{rs}");
    same_tree(&s, &o);
    // restauration vers un dossier non vide : refusée
    let f = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"restore","repo":"{}","id":"{id}","target":"{}"}}"#, r.display(), o.display()), "failed"); assert!(f.contains("restore.target_not_empty"), "{f}");
    drop(c.stdin.take()); c.wait().unwrap();
}

#[test] fn purge_via_protocol_needs_confirmed_plan_and_user() {
    let t = tempfile::tempdir().unwrap(); let (s, r) = (t.path().join("src"), t.path().join("repo")); fs::create_dir_all(&s).unwrap();
    let mut c = spawn(); let mut rd = BufReader::new(c.stdout.take().unwrap());
    for i in 0..2 { fs::write(s.join("a"), format!("v{i}")).unwrap(); ask(&mut c, &mut rd, &backup_cmd(&s, &r), "done"); std::thread::sleep(std::time::Duration::from_millis(1100)); }
    let pl = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"purge_plan","repo":"{}","keep":1}}"#, r.display()), "purge_plan");
    let v: serde_json::Value = serde_json::from_str(&pl).unwrap(); let pid = v["plan"]["plan_id"].as_str().unwrap().to_string();
    assert_eq!(v["plan"]["remove"].as_array().unwrap().len(), 1);
    // sans « user_present » : différé
    let f = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"purge_execute","repo":"{}","keep":1,"plan_id":"{pid}"}}"#, r.display()), "failed"); assert!(f.contains("purge.deferred"), "{f}");
    let ok = ask(&mut c, &mut rd, &format!(r#"{{"cmd":"purge_execute","repo":"{}","keep":1,"plan_id":"{pid}","user_present":true}}"#, r.display()), "purged"); assert!(ok.contains("\"removed_snapshots\":1"), "{ok}");
    drop(c.stdin.take()); c.wait().unwrap();
}
