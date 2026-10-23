//! Présentation des événements : code + args -> texte. L'historique n'est jamais modifié.
use core_types::Event;
use serde_json::Value;

const FR: &str = include_str!("../locales/fr.json");
const EN: &str = include_str!("../locales/en.json");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang { Fr, En }

/// Mode « Système » : français pour toute variante `fr*`, anglais sinon (spec §4).
pub fn resolve_system(locale: &str) -> Lang {
    let l = locale.to_ascii_lowercase();
    if l == "fr" || l.starts_with("fr-") || l.starts_with("fr_") { Lang::Fr } else { Lang::En }
}

fn catalog(lang: Lang) -> Value { serde_json::from_str(if lang == Lang::Fr { FR } else { EN }).expect("catalogue valide") }

/// Rend un événement. Code inconnu : renvoie le code brut (jamais de panique, jamais de perte).
pub fn render(lang: Lang, ev: &Event) -> String {
    let cat = catalog(lang);
    let Some(entry) = cat.get(&ev.code) else { return ev.code.clone() };
    let plural = ev.args.get("count").and_then(|c| c.parse::<i64>().ok())
        .map(|n| if n == 1 || n == 0 && lang == Lang::Fr { "one" } else { "other" }).unwrap_or("other");
    let tpl = entry.get(plural).or_else(|| entry.get("other")).and_then(Value::as_str).unwrap_or(&ev.code);
    ev.args.iter().fold(tpl.to_string(), |s, (k, v)| s.replace(&format!("{{{k}}}"), v))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn system_lang() {
        for l in ["fr", "fr-FR", "fr_CA", "fr-BE"] { assert_eq!(resolve_system(l), Lang::Fr); }
        for l in ["en-US", "es-ES", "de", "frq"] { assert_eq!(resolve_system(l), Lang::En); }
    }
    #[test] fn plurals() {
        let one = Event::new("backup.file_copied").arg("count", 1);
        let many = Event::new("backup.file_copied").arg("count", 5);
        assert_eq!(render(Lang::Fr, &one), "1 fichier sauvegardé");
        assert_eq!(render(Lang::En, &many), "5 files backed up");
    }
    #[test] fn unknown_code_kept() { assert_eq!(render(Lang::En, &Event::new("x.y")), "x.y"); }
    #[test] fn catalogs_have_same_keys() {
        let (f, e) = (catalog(Lang::Fr), catalog(Lang::En));
        let k = |v: &Value| { let mut k: Vec<_> = v.as_object().unwrap().keys().cloned().collect(); k.sort(); k };
        assert_eq!(k(&f), k(&e));
    }
}
