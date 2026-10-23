# Contenu du kit

Ce kit contient la version consolidée du cahier des charges et les cinq éléments demandés : procédure Git, README, modèle de configuration, commandes Rust et arborescence Markdown. Il comprend aussi un guide des prérequis actualisé.

Le nom du projet reste provisoire. Les sources applicatives, le workspace Cargo et le projet Xcode ne sont pas encore créés. Aucun dépôt Git distant n’a été créé ou modifié.

Ordre de lecture :
1. README.md
2. docs/GUIDE_PREREQUIS.md
3. docs/CONFIGURATION_GIT.md
4. docs/STRUCTURE_PROJET.md
5. docs/CONFIGURATION.md et config/config.example.json
6. docs/COMPILATION.md
7. PROMPT_FINAL.txt à transmettre à l’IA.

## Clarifications apportées au prompt consolidé

- Langue automatique : français pour un système français, anglais sinon, avec choix manuel.
- Source en lecture seule pendant la sauvegarde ; restauration à l’emplacement original possible uniquement après accord explicite.
- Rétention proposée automatiquement mais purge confirmée en V1 : pas de suppression silencieuse contradictoire avec les garde-fous.
- Reprise et vérification restent bien des exigences de V1.
- Les versions datées doivent représenter des états restaurables, pas de simples dossiers de fichiers modifiés.
- Les adaptations macOS et limites de permissions sont explicites.

Les commandes sont des instructions pour le futur environnement macOS, pas des opérations réalisées pendant la préparation des fichiers. Le JSON est syntaxiquement validé, mais aucun moteur applicatif ne le charge encore.
