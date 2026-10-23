//! Types partagés : états, événements à code stable, versions de schéma/protocole.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version du protocole moteur <-> interface. À incrémenter à toute rupture.
pub const PROTOCOL_VERSION: u32 = 1;

/// États d'une tâche (spec §8). `dernière tentative` et `dernière réussite` sont stockées à part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState { NeverRun, Pending, Running, Succeeded, Partial, Interrupted, Failed }

/// Événement technique : code stable + arguments structurés (jamais de texte traduit stocké).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub code: String,
    pub args: BTreeMap<String, String>,
}

impl Event {
    pub fn new(code: &str) -> Self { Self { code: code.into(), args: BTreeMap::new() } }
    pub fn arg(mut self, k: &str, v: impl ToString) -> Self { self.args.insert(k.into(), v.to_string()); self }
}

/// Lecture d'une version de format persistant : refuse proprement le futur inconnu.
pub fn check_schema(found: u32, supported_max: u32) -> Result<(), Event> {
    if found > supported_max {
        Err(Event::new("schema.unknown_future").arg("found", found).arg("max", supported_max))
    } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn future_schema_refused() { assert!(check_schema(2, 1).is_err()); assert!(check_schema(1, 1).is_ok()); }
    #[test] fn event_roundtrip() {
        let e = Event::new("backup.file_copied").arg("count", 3);
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(serde_json::from_str::<Event>(&s).unwrap(), e);
    }
    #[test] fn state_serializes_snake() {
        assert_eq!(serde_json::to_string(&TaskState::NeverRun).unwrap(), "\"never_run\"");
    }
}
