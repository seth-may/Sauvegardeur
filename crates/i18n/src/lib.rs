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
    fn placeholders(s: &str) -> Vec<String> {
        let mut v: Vec<String> = s.split('{').skip(1).filter_map(|x| x.split('}').next()).map(String::from).collect(); v.sort(); v
    }
    #[test] fn placeholders_match_between_languages() {
        let (f, e) = (catalog(Lang::Fr), catalog(Lang::En));
        for (k, fv) in f.as_object().unwrap() { for (form, ft) in fv.as_object().unwrap() {
            let et = e[k][form].as_str().unwrap_or_else(|| panic!("{k}/{form} absent en EN"));
            assert_eq!(placeholders(ft.as_str().unwrap()), placeholders(et), "{k}/{form}");
        } }
    }
    /// Tout code d'événement émis dans le code source doit avoir une traduction fr ET en.
    #[test] fn every_emitted_code_is_translated() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let cat = catalog(Lang::Fr); let mut found = 0;
        for krate in std::fs::read_dir(&root).unwrap().flatten() {
            if krate.file_name() == "i18n" { continue }
            let mut stack = vec![krate.path().join("src")];
            while let Some(d) = stack.pop() { for f in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = f.path(); if p.is_dir() { stack.push(p); continue }
                if p.extension().is_none_or(|x| x != "rs") { continue }
                let src = std::fs::read_to_string(&p).unwrap();
                for pat in ["Event::new(\"", "code: \"", "Err(err(\"", "err(\""] { for part in src.split(pat).skip(1) {
                    let code = part.split('"').next().unwrap();
                    if !code.contains('.') || code.contains(' ') { continue }
                    found += 1; assert!(cat.get(code).is_some(), "code sans traduction : {code} ({})", p.display());
                } }
            } }
        }
        assert!(found > 20, "scan du code source suspect ({found})");
    }
}
