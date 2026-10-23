# Structure cible du projet

Cette arborescence est une proposition d’organisation. Le kit fournit les documents et modèles, pas les sources ou projets de compilation ci-dessous.

```text
projet-sauvegarde/
├── README.md
├── PROMPT_FINAL.txt
├── .gitignore
├── Cargo.toml                      # workspace, à créer
├── Cargo.lock                      # généré puis versionné
├── config/
│   ├── config.example.json         # modèle versionné
│   └── config.local.json           # copie locale ignorée
├── crates/
│   ├── backup-core/                # règles métier, orchestration et interfaces
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── tests/
│   ├── backup-storage/             # état persistant, manifestes et migrations
│   ├── backup-platform-macos/      # adaptateurs propres à macOS
│   └── backup-bridge/              # frontière Rust–Swift, transport à décider
├── apps/
│   └── macos/
│       ├── Sauvegarde.xcodeproj/
│       ├── Sauvegarde/
│       │   ├── App/
│       │   ├── Views/
│       │   ├── ViewModels/
│       │   ├── Services/           # adaptateur UI du moteur, pas de copie ici
│       │   └── Resources/          # catalogues de chaînes et assets originaux
│       ├── SauvegardeTests/
│       └── SauvegardeUITests/
├── docs/
│   ├── GUIDE_PREREQUIS.md
│   ├── CONFIGURATION_GIT.md
│   ├── CONFIGURATION.md
│   ├── COMPILATION.md
│   ├── STRUCTURE_PROJET.md
│   ├── architecture/               # contrats et formats
│   ├── adr/                        # décisions d’architecture datées
│   ├── fr/                         # guide utilisateur français
│   └── en/                         # guide utilisateur anglais
├── tests/
│   ├── fixtures/                   # petites données factices versionnées
│   ├── scenarios/                  # cas d’acceptation et restauration
│   └── sandbox/                    # données générées ignorées
├── scripts/                        # scripts créés et relus ultérieurement
├── .local/                         # état et bac à sable locaux ignorés
└── build/                          # produits générés ignorés
```

## Dépendances et responsabilités

- backup-core ne dépend pas de SwiftUI. Définir les contrats d’accès au stockage et au système sans imposer leur implémentation.
- backup-storage implémente la persistance et les migrations ; éviter toute dépendance circulaire avec le noyau.
- backup-platform-macos isole les opérations spécifiques et les limites de permissions.
- backup-bridge traduit commandes, événements, annulation et erreurs entre Rust et Swift.
- Les ViewModels pilotent l’état d’affichage, pas les règles de sauvegarde.
- Les tests inter-composants et de bout en bout complètent les tests unitaires propres à chaque crate.

Ne créer les crates supplémentaires que lorsqu’une responsabilité concrète le justifie. Aucun système de plugins dynamiques ni infrastructure cloud n’est exigé pour la V1.
