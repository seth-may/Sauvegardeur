# Commandes de compilation et de validation

## 1. Préparer le Mac

Installer Xcode complet, l’ouvrir et terminer l’installation de ses composants. Vérifier le répertoire développeur actif :

```bash
xcode-select -p
xcodebuild -version
rustc --version
cargo --version
rustup --version
```

Si nécessaire et si Xcode est bien installé dans /Applications :

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

Cette commande change le Xcode actif sur la machine. Ne pas la lancer si le chemin ne correspond pas à l’installation.

Si Rust n’est pas installé, suivre https://rustup.rs/ puis ouvrir un nouveau terminal. Installer les outils de qualité :

```bash
rustup component add rustfmt clippy
```

Choisir ensuite une version Rust précise pour les versions reproductibles ; le kit n’impose pas un numéro non testé.

## 2. Créer le premier module Rust

À exécuter une seule fois, depuis la racine du projet, si crates/backup-core n’existe pas :

```bash
mkdir -p crates
cargo new --lib --vcs none crates/backup-core
```

Créer ensuite le fichier Cargo.toml à la racine avec :

```toml
[workspace]
members = ["crates/backup-core"]
resolver = "2"
```

Ne pas écraser un Cargo.toml existant. Ce workspace minimal contient uniquement le noyau ; les autres crates de l’arborescence seront ajoutées à mesure de leur création.

## 3. Commandes usuelles — depuis la racine du workspace

```bash
# Vérification rapide sans produire l’exécutable final
cargo check --workspace --all-targets

# Compilation de développement
cargo build --workspace

# Compilation optimisée
cargo build --workspace --release

# Tests unitaires, intégration et documentation selon les cibles présentes
cargo test --workspace

# Vérification puis application du formatage
cargo fmt --all -- --check
cargo fmt --all

# Analyse statique ; les avertissements deviennent des erreurs
cargo clippy --workspace --all-targets -- -D warnings

# Documentation Rust
cargo doc --workspace --no-deps
```

Le Cargo.lock sera généré et doit être versionné pour cette application. Ensuite, utiliser notamment :

```bash
cargo build --workspace --release --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Ne pas ajouter systématiquement --all-features : certaines fonctions futures pourront être incompatibles entre elles ou dépendantes de la plateforme. Définir une matrice de tests explicite.

Cargo seul ne crée pas une application macOS distribuable. Une bibliothèque Rust compilée n’est pas automatiquement reliée à SwiftUI.

## 4. Architectures macOS

Sur un Mac Apple Silicon, préparer éventuellement les deux cibles :

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --workspace --release --target aarch64-apple-darwin --locked
cargo build --workspace --release --target x86_64-apple-darwin --locked
```

Les résultats vont sous target/<cible>/release/. La compatibilité croisée dépend aussi du SDK, des dépendances natives et du pont choisi. Ces commandes ne garantissent pas une distribution Universal ; les deux architectures doivent être assemblées et testées si elles sont annoncées comme prises en charge.

## 5. SwiftUI et Xcode — après création du projet

Créer dans Xcode une application macOS SwiftUI, en visant l’arborescence apps/macos/Sauvegarde.xcodeproj. Définir le bundle ID, la version minimale de macOS, la signature et un scheme partagé nommé Sauvegarde. Ce projet n’est pas fourni dans ce kit.

Depuis la racine, adapter les noms si nécessaire :

```bash
xcodebuild -list -project apps/macos/Sauvegarde.xcodeproj
xcodebuild -project apps/macos/Sauvegarde.xcodeproj \
  -scheme Sauvegarde -configuration Debug \
  -destination 'platform=macOS' -derivedDataPath build/DerivedData build
```

Une fois les cibles de tests créées et rattachées au scheme :

```bash
xcodebuild -project apps/macos/Sauvegarde.xcodeproj \
  -scheme Sauvegarde -destination 'platform=macOS' \
  -derivedDataPath build/DerivedData test
```

Pour une compilation Release :

```bash
xcodebuild -project apps/macos/Sauvegarde.xcodeproj \
  -scheme Sauvegarde -configuration Release \
  -destination 'platform=macOS' -derivedDataPath build/DerivedData build
```

Le pont Rust–Swift doit être implémenté, ses artefacts reliés et ses étapes de compilation configurées avant qu’une application intégrée puisse fonctionner. Ne pas inventer de commande cargo run : aucune CLI n’est fournie ici.

## 6. Distribution et validation

Définir séparément : signature, profils éventuels, notarisation, emballage et mises à jour. Un build Release local n’équivaut pas à une application signée/notarisée prête à distribuer. Ne pas committer les clés de signature.

Avant toute annonce de version : tester restauration, interruptions, migrations et langues sur les systèmes réellement pris en charge. Les commandes de ce guide n’ont pas été exécutées dans un environnement macOS lors de la préparation du kit.

## Sources techniques

- Cargo : https://doc.rust-lang.org/cargo/commands/
- Clippy : https://doc.rust-lang.org/stable/clippy/usage.html
- Xcode : https://developer.apple.com/documentation/xcode/xcode-command-line-tool-reference
- Schemes : https://developer.apple.com/documentation/xcode/customizing-the-build-schemes-for-a-project/
