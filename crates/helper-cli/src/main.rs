//! Helper local : protocole JSON-lines (stdin -> stdout), sans réseau.
//! Commandes : hello, ping, backup{source,repo}, cancel. Un seul travail à la fois.
use core_types::{Event, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Request { Hello, Ping, Cancel, Backup { source: String, repo: String },
    Verify { repo: String, id: Option<String>, #[serde(default)] full: bool },
    Restore { repo: String, id: String, subpath: Option<String>, target: String },
    List { repo: String },
    PurgePlan { repo: String, keep: usize },
    /// `user_present` vaut faux par défaut : une tâche automatique ne purge jamais.
    PurgeExecute { repo: String, keep: usize, plan_id: String, #[serde(default)] user_present: bool } }

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Hello { protocol: u32 }, Pong, Error { code: String },
    Progress { total: u64, processed: u64, copied: u64, bytes: u64 },
    Done { id: String, status: store::SnapshotStatus, scanned: u64, copied: u64, reused: u64, bytes: u64, errors: Vec<Event> },
    Failed { event: Event },
    Verified { id: String, full: bool, problems: Vec<Event> },
    Restored { count: u64 },
    PurgePlan { plan: retention::PurgePlan },
    Purged { report: retention::PurgeReport },
    Snapshots { ids: Vec<String>, latest_success: Option<String> },
}

type Out = Arc<Mutex<std::io::Stdout>>;
fn send(out: &Out, r: &Response) {
    let mut o = out.lock().unwrap();
    let _ = writeln!(o, "{}", serde_json::to_string(r).unwrap()); let _ = o.flush();
}

fn main() {
    let out: Out = Arc::new(Mutex::new(std::io::stdout()));
    let busy = Arc::new(AtomicBool::new(false));
    let cancel = Arc::new(AtomicBool::new(false));
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        match serde_json::from_str::<Request>(&line) {
            Ok(Request::Hello) => send(&out, &Response::Hello { protocol: PROTOCOL_VERSION }),
            Ok(Request::Ping) => send(&out, &Response::Pong),
            Ok(Request::Cancel) => cancel.store(true, Ordering::Relaxed),
            Ok(req @ (Request::Backup { .. } | Request::Verify { .. } | Request::Restore { .. } | Request::PurgePlan { .. } | Request::PurgeExecute { .. })) => {
                if busy.swap(true, Ordering::SeqCst) { send(&out, &Response::Failed { event: Event::new("job.busy") }); continue; }
                cancel.store(false, Ordering::Relaxed);
                let (o, b, c) = (out.clone(), busy.clone(), cancel.clone());
                std::thread::spawn(move || {
                    let resp = run_job(req, &o, &c).unwrap_or_else(|event| Response::Failed { event });
                    b.store(false, Ordering::SeqCst); // libérer AVANT d'annoncer la fin : le client peut enchaîner
                    send(&o, &resp);
                });
            }
            Ok(Request::List { repo }) => match store::Repo::open_or_init(repo.as_ref()) {
                Ok(r) => send(&out, &Response::Snapshots { ids: r.list_snapshots(), latest_success: r.latest_success() }),
                Err(event) => send(&out, &Response::Failed { event }),
            },
            Err(_) => send(&out, &Response::Error { code: "protocol.bad_request".into() }),
        }
    }
    // Fin de stdin : laisser le travail en cours se terminer proprement (ou être tué par le parent).
    while busy.load(Ordering::SeqCst) { std::thread::sleep(std::time::Duration::from_millis(20)); }
}

fn run_job(req: Request, o: &Out, c: &AtomicBool) -> Result<Response, Event> {
    Ok(match req {
        Request::Backup { source, repo } => {
            let r = store::backup::run_backup_with(source.as_ref(), repo.as_ref(), c, &mut |p| {
                send(o, &Response::Progress { total: p.total, processed: p.processed, copied: p.copied, bytes: p.bytes })
            })?;
            Response::Done { id: r.id, status: r.status, scanned: r.scanned, copied: r.copied, reused: r.reused, bytes: r.bytes, errors: r.errors }
        }
        Request::Verify { repo, id, full } => {
            let r = store::Repo::open_or_init(repo.as_ref())?;
            let id = id.or_else(|| r.latest_success()).ok_or_else(|| Event::new("snapshot.unreadable").arg("path", "latest_success"))?;
            let mode = if full { store::VerifyMode::Full } else { store::VerifyMode::Quick };
            let problems = r.verify_with(&id, mode, c)?;
            Response::Verified { id, full, problems }
        }
        Request::Restore { repo, id, subpath, target } => {
            let r = store::Repo::open_or_init(repo.as_ref())?;
            let count = store::restore::restore(&r, &id, subpath.as_deref(), target.as_ref())?;
            Response::Restored { count }
        }
        Request::PurgePlan { repo, keep } => {
            let r = store::Repo::open_or_init(repo.as_ref())?;
            Response::PurgePlan { plan: retention::plan(&r, keep)? }
        }
        Request::PurgeExecute { repo, keep, plan_id, user_present } => {
            let r = store::Repo::open_or_init(repo.as_ref())?;
            Response::Purged { report: retention::execute(&r, keep, &plan_id, user_present)? }
        }
        _ => unreachable!(),
    })
}
