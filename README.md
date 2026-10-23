# Projet Sauvegarde — Rust + SwiftUI

Nom de travail, à remplacer avant publication.

## Objectif

Créer une application native macOS de sauvegarde, simple, fiable et bilingue, inspirée des principes d’ergonomie de Carbon Copy Cloner, avec une identité visuelle originale. Le projet n’est ni affilié à Carbon Copy Cloner ni présenté comme un produit équivalent.

Le moteur sera écrit en Rust ; l’interface native en SwiftUI. L’architecture doit permettre une montée en puissance progressive sans compromettre la restauration des anciennes sauvegardes.

## État du projet

Phase de cadrage. Ce dossier contient des documents et des modèles, pas une application opérationnelle. Aucun moteur de sauvegarde, pont Rust–Swift ou projet Xcode compilable n’est livré dans ce kit. Les fonctions ci-dessous sont des objectifs, non des fonctions disponibles.

## Objectifs de la V1

- Tâches de sauvegarde de dossiers locaux vers une destination locale ou un disque externe.
- Copie incrémentielle, historique daté et restauration.
- Sauvegardes manuelles, quotidiennes, hebdomadaires et au branchement du disque.
- États explicites, vérification, reprise après interruption et conservation des versions.
- Français si la langue système est française ; anglais sinon.
- Réglage Système / Français / English, appliqué à l’interface, l’aide, les messages et la documentation.

La V1 exclut le cloud, le NAS, le chiffrement du dépôt de sauvegarde et le clonage système.

## Principes de sécurité

- La sauvegarde lit la source sans jamais la modifier.
- Une restauration vers la source est une opération distincte, explicitement autorisée.
- Aucun essai initial sur des données personnelles irremplaçables.
- Une sauvegarde incomplète n’est jamais annoncée comme réussie.
- Une ancienne version ne doit pas dépendre de fichiers susceptibles d’être modifiés en place.
- Pas de suppression sans autorisation ; la dernière sauvegarde réussie reste protégée.
- Les scénarios de restauration font partie des critères de validation.

## Architecture prévue

```text
SwiftUI → adaptateur de communication → moteur Rust
                                      ├── tâches et configuration
                                      ├── copie et vérification
                                      ├── historique et restauration
                                      ├── rétention et planification
                                      └── événements structurés et diagnostics
```

Le transport entre Rust et Swift reste à choisir dans une décision d’architecture : bibliothèque avec pont FFI, processus local ou autre mécanisme justifié. Ne pas créer un service réseau par défaut.

## Documents

- `docs/GUIDE_PREREQUIS.md` : préparation et points de validation.
- `docs/CONFIGURATION_GIT.md` : création du dépôt et premier envoi distant.
- `docs/COMPILATION.md` : commandes Rust et macOS.
- `docs/STRUCTURE_PROJET.md` : arborescence cible et responsabilités.
- `config/config.example.json` : modèle de configuration de développement.
- `docs/CONFIGURATION.md` : signification du modèle et limites de sécurité.
- `PROMPT_FINAL.txt` : cahier des charges à transmettre à l’IA.

## Démarrage

1. Lire le guide des prérequis et la procédure Git.
2. Choisir le nom, l’identifiant de bundle, la version minimale de macOS et les architectures cibles.
3. Faire valider l’architecture et la stratégie d’historique avant le code.
4. Créer les modules Rust et le projet Xcode selon la documentation.
5. Exécuter les tests uniquement dans le bac à sable de développement.

Le fichier de configuration fourni est un contrat proposé : aucun programme ne le lit encore. Ne pas lui confier de données réelles.

## Évolution

Prévoir des interfaces et formats versionnés, des migrations testées et une documentation de restauration indépendante de l’interface. Les destinations réseau, le chiffrement, les snapshots et les fonctions système seront étudiés séparément, sans promesse préalable de faisabilité.

## Contributions et licence

La licence n’est pas encore choisie. Décider des droits de diffusion avant une publication publique et vérifier les licences des dépendances. Conserver les secrets, identifiants de signature et données de test privées hors du dépôt.
