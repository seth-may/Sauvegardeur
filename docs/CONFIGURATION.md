# Configuration initiale

## Choix du format

Le modèle est `config/config.example.json`, plutôt que `config.js` : le projet n’a pas de runtime JavaScript prévu. Aucun `.env` n’est nécessaire à ce stade puisqu’aucun secret ni service externe n’est requis. Ce JSON est une proposition de contrat, pas un chargeur de configuration implémenté.

Depuis la racine :

```bash
mkdir -p config .local/sandbox/source .local/sandbox/destination .local/logs .local/state
cp config/config.example.json config/config.local.json
```

Ne lancer `cp` qu’à la première configuration : il écraserait une configuration locale existante.

## Paramètres

| Clé | Sens attendu |
|---|---|
| schema_version | Version du format, contrôlée avant toute lecture/migration |
| application.environment | Environnement ; ici development |
| application.language | system, fr ou en ; system → fr pour un système français, en sinon |
| development.sandbox_root | Racine des données factices, relative à la racine du dépôt |
| development.allow_only_sandbox_paths | Refuser toute lecture/écriture de test hors du bac à sable |
| development.simulate_by_default | Simulation par défaut : pas de copie ni suppression |
| development.allow_scheduled_runs | Planification désactivée en développement par défaut |
| development.allow_volume_triggers | Déclenchement au branchement désactivé par défaut |
| backup_defaults.verify_after_copy | checksum : empreinte après copie ; algorithme à arrêter dans une ADR |
| backup_defaults.retain_successful_versions | Nombre cible de versions réussies conservées |
| backup_defaults.preserve_latest_successful | Protection de la dernière sauvegarde réussie |
| backup_defaults.require_confirmation_before_deletion | Confirmation de chaque purge en V1 |
| backup_defaults.restore_to_new_location_by_default | Restauration non destructive par défaut |
| diagnostics.level | Niveau de journalisation |
| diagnostics.directory | Emplacement local des diagnostics |
| diagnostics.redact_personal_paths | Masquage des chemins personnels dans les exports de diagnostic |
| storage.state_directory | Emplacement local de l’état de développement |

## Contrat à implémenter

- Valider le schéma et les valeurs ; une configuration invalide interdit les écritures.
- Résoudre les chemins par rapport à une racine explicite, pas au hasard du répertoire courant.
- Appliquer les contrôles de confinement aussi après résolution des liens symboliques.
- Vérifier les identités source/destination et refuser une destination incluse dans la source, ou inversement.
- Ne pas interpréter la présence de ce fichier comme une garantie de sécurité : les contrôles doivent être codés et testés.
- Ne jamais livrer les chemins et paramètres de développement comme réglages de production.
- Les tâches utilisateur seront stockées dans un format versionné distinct ; aucun dossier personnel n’est préconfiguré dans ce modèle.
- Ne stocker aucun mot de passe, jeton ou clé privée dans le JSON.

Les langues stockées sont des identifiants stables. Les événements de journal contiennent des codes et arguments neutres ; leur texte est rendu dans la langue choisie, sans réécrire l’historique.
