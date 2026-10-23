# Guide des prérequis — Rust + SwiftUI

## 1. Cadrage avant installation

- [ ] Choisir le nom de travail, le nom de dépôt et l’identifiant de bundle.
- [ ] Déterminer la version minimale de macOS et les architectures visées.
- [ ] Définir les formats de volumes pris en charge et leurs limites.
- [ ] Définir le périmètre V1 et ses critères d’acceptation.
- [ ] Valider l’architecture et le contrat Rust–Swift avant la génération massive de code.
- [ ] Choisir les conditions de diffusion et la licence avant publication.

Les compatibilités et performances ne sont pas encore établies : elles devront être vérifiées sur les versions et machines retenues.

## 2. Matériel

Prévoir un Mac compatible avec le Xcode choisi, de l’espace libre pour les outils et les jeux de tests, une connexion pour installer les dépendances et au moins un support externe réservé aux essais.

À titre de confort et non d’exigence officielle : viser 16 Go de RAM et environ 100 Go libres, à adapter à la taille des données de test. Un second volume ou support facilite les essais de restauration et de déconnexion. Un second Mac n’est pas obligatoire pour commencer ; il sera utile pour la validation indépendante.

Ne pas utiliser le disque contenant l’unique copie de données réelles pour les essais destructifs.

## 3. Outils

- Xcode complet et ses outils de ligne de commande.
- Rust via rustup, Cargo, rustfmt et Clippy.
- Git et un éditeur au choix ; Xcode pour l’application SwiftUI.
- Dépôt distant privé recommandé au départ, sans y placer de secrets.
- Aucun outil JavaScript, Node.js, Docker ou service cloud n’est requis par le socle proposé.

Voir COMPILATION.md pour les contrôles et commandes. Définir une chaîne d’outils reproductible après les premiers essais concluants ; ne pas changer toutes les versions automatiquement en cours de jalon.

## 4. Connaissances minimales pour piloter une IA

Il n’est pas nécessaire de maîtriser tout Rust avant de commencer, mais il faut pouvoir relire une modification, lancer les tests et identifier une opération dangereuse.

- Terminal : dossiers, chemins, droits et lecture des sorties.
- Git : diff, commit, branche, revue et retour à un état connu sans perte involontaire.
- Rust : Result/Option, ownership, modules, tests et gestion des fichiers.
- SwiftUI : état d’interface, exécution asynchrone et affichage des erreurs.
- Sauvegarde : différence entre copie, synchronisation, historique et restauration.
- macOS : autorisations utilisateur et opérations réservées au système.

L’IA produit des propositions, pas une preuve de fiabilité. Faire relire les parties critiques si nécessaire et exiger les sorties de tests réellement exécutés.

## 5. Décisions bloquantes avant le moteur

### Historique

Choisir et documenter comment une sauvegarde datée présente un état restaurable complet tout en ne recopiant que les changements. Définir les manifestes, les références de données, l’immuabilité des versions et les règles de purge. Un dossier daté de seuls fichiers modifiés n’est pas à lui seul une vue complète restaurable.

### Frontière native

Choisir le pont Rust–Swift avec ses contraintes de mémoire, concurrence, annulation et erreurs. La planification ou les événements de volume peuvent nécessiter des adaptateurs macOS ; le noyau conserve les règles métier.

### Permissions et processus de fond

Documenter la sandbox éventuelle, l’accès aux dossiers sélectionnés, la persistance de cet accès, les éventuelles autorisations supplémentaires et le comportement lorsque l’application est fermée, le Mac en veille ou le volume absent. Ne pas demander des droits administrateur par défaut.

### Métadonnées et système de fichiers

Définir la prise en charge des permissions, liens symboliques, liens physiques, attributs étendus, ACL et collisions de casse. Refuser ou signaler les conversions non fidèles ; ne pas promettre une copie complète des métadonnées sur tous les formats de destination.

### Persistance

Choisir SQLite ou un format structuré selon les besoins. Versionner configurations, manifestes, contrats d’événements et migrations. Prévoir copie de sécurité et reprise après une migration interrompue.

## 6. Jeu de tests

Créer uniquement des données factices : fichiers vides, textes, fichiers volumineux, arborescences profondes, noms accentués, Unicode, caractères spéciaux et liens. Simuler ou provoquer dans un environnement isolé les droits refusés, manque d’espace, déconnexion et interruption.

Contrôler : première copie, changements, suppressions à la source, versions historiques inchangées, restauration complète et partielle, collisions de noms, reprise, purge, changement de langue et migrations.

La reprise et la vérification sont des exigences de V1, pas seulement des améliorations futures.

## 7. Règles de non-destruction

- Sources en lecture seule pendant la sauvegarde.
- Restaurer ailleurs par défaut ; vers la source seulement après autorisation explicite.
- Refuser les chemins source/destination identiques ou imbriqués.
- Vérifier l’identité du disque, pas seulement son nom ou son chemin de montage.
- Ne pas transformer une erreur de lecture en suppression de la copie sauvegardée.
- Ne jamais purger une version dont dépend une autre version conservée.
- Aucun succès déclaré tant qu’une erreur requise n’est pas résolue.
- La confirmation de purge est obligatoire en V1 ; une purge planifiée peut donc être différée.

## 8. Bilinguisme

Par défaut : français pour une langue système française, anglais sinon. Réglage persistant Système / Français / English. Couvrir interface, aide, documentation, notifications et rendus de journaux. Les données techniques restent indépendantes de la langue. Tester l’absence de clés manquantes, les pluriels, dates, nombres et textes tronqués.

## 9. Ordre des jalons

1. Outils, dépôt, documentation et décisions bloquantes.
2. Noyau Rust avec simulation et tests sur fichiers factices.
3. Copie sûre, historique cohérent, vérification et restauration.
4. Pont natif et interface SwiftUI bilingue.
5. Reprise, planification, rétention protégée et tests de bout en bout.
6. Packaging, signature, documentation et essais sur les systèmes cibles.

Chaque jalon doit produire un résultat vérifiable avant le suivant. NAS, cloud, chiffrement et fonctions système restent hors V1.

## 10. Avant des données réelles

- [ ] Tests de non-destruction concluants et relus.
- [ ] Restauration vérifiée par comparaison du contenu.
- [ ] Déconnexion, interruption et espace insuffisant gérés.
- [ ] Anciennes versions et migrations testées.
- [ ] Français et anglais complets, documentation comprise.
- [ ] Autorisations minimales expliquées et révocables.
- [ ] Une autre sauvegarde indépendante des données existe déjà.
- [ ] Aucune fonction annoncée sans test correspondant.

La réussite d’une compilation ne valide ni l’intégrité ni la restaurabilité d’une sauvegarde.
