# État (M5bis) — Rust uniquement, testé sous Linux
Décisions : helper local JSON-lines · dépôt d'objets + manifestes · tous FS (APFS/HFS+ recommandés) · hors App Store · agent launchd · macOS 14+ (à valider) · binaire universel (à construire sous macOS) · nom : Sauvegardeur.
Non fait : tâches/config persistantes, planification, détection de volume, SwiftUI/Xcode, xattrs/ACL, docs utilisateur fr/en, signature.
Commandes : `cargo test --workspace` (≈45 s) · `cargo build --release -p helper-cli`.
