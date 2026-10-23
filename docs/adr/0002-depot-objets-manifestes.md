# ADR 0002 — Historique : dépôt d'objets + manifestes complets
Décision : objets immuables adressés par hash ; un manifeste complet par point ; écriture temp -> rename atomique.
Destinations : tous systèmes de fichiers acceptés ; APFS/HFS+ recommandés (métadonnées, xattrs, liens). Perte de fidélité signalée.
Compromis : dépôt non navigable au Finder (restauration via l'app).
