//! Helper local : protocole JSON-lines (stdin -> stdout), sans réseau. M0 : `hello` et `ping` seulement.
use core_types::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Request { Hello, Ping }

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response { Hello { protocol: u32 }, Pong, Error { code: String } }

fn handle(line: &str) -> Response {
    match serde_json::from_str::<Request>(line) {
        Ok(Request::Hello) => Response::Hello { protocol: PROTOCOL_VERSION },
        Ok(Request::Ping) => Response::Pong,
        Err(_) => Response::Error { code: "protocol.bad_request".into() },
    }
}

fn main() {
    let out = std::io::stdout();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let mut o = out.lock();
        let _ = writeln!(o, "{}", serde_json::to_string(&handle(&line)).unwrap());
        let _ = o.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn hello() { assert!(matches!(handle(r#"{"cmd":"hello"}"#), Response::Hello { protocol: 1 })); }
    #[test] fn bad() { assert!(matches!(handle("zzz"), Response::Error { .. })); }
}
