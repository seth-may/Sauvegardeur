# Cadrage pré-code — Sauvegardeur (nom provisoire)
Statut : proposition à valider. Aucun code écrit. Versions d'outils/macOS : **non vérifiées**, à confirmer sur ta machine.

## 1. Questions bloquantes (réponds A/B)
1. **Pont Rust–Swift** : (A) processus helper local Rust, dialogue JSON-lines via stdin/stdout, embarqué dans l'app *(recommandé)* ; (B) bibliothèque statique + FFI C.
2. **Format de l'historique** : (A) dépôt par objets (contenu adressé par hash) + manifestes *(recommandé)* ; (B) arbres datés à liens physiques (navigables dans le Finder, mais exigent APFS/HFS+ en destination).
3. **Disques de destination visés** : APFS/HFS+ uniquement, ou aussi exFAT/FAT ? (impacte fidélité des métadonnées)
4. **Distribution** : usage perso hors App Store (Developer ID, non sandboxé + Accès complet au disque) *(recommandé)* ou App Store (sandbox, plus contraignant) ?
5. **Planification en arrière-plan** : V1 avec agent launchd (SMAppService) pour quotidien/hebdo/branchement disque, ou V1 « uniquement app ouverte » et agent en V1.1 ?
6. **Macs ciblés** : Apple Silicon seul ou aussi Intel ? macOS minimum (hypothèse à valider : 14+) ?
7. **Nom définitif** (à vérifier : pas de conflit de marque).

## 2. Hypothèses
- Usage perso, volumes de quelques centaines de Go, un seul utilisateur.
- Source toujours en lecture seule (ouverture O_RDONLY, aucune écriture/xattr côté source).
- Pas de snapshot APFS en V1 : pas d'image cohérente instantanée ; un fichier modifié pendant la lecture est détecté (taille/mtime avant-après + hash) et relu ou marqué « changé pendant copie ».

## 3. Architecture (workspace Cargo, dépendances → vers le bas)
```
core-types   (modèles, codes d'erreur/événements stables, versions de schéma)
  ├ fs-scan      (parcours, normalisation Unicode, collisions de casse, liens)
  ├ store        (dépôt d'objets, manifestes, écriture temp → fsync → rename atomique)
  ├ verify       (hash, contrôle sans copie)
  ├ retention    (calcul des versions purgeables, plan de purge)
  ├ config       (tâches JSON versionné, migrations)
  └ journal      (événements structurés, export, masquage des chemins)
engine       (orchestration : backup, resume, restore, verify, purge-plan ; annulation)
helper-cli   (binaire : protocole JSON-lines versionné ↔ engine)
macos/App    (SwiftUI : vues, état d'affichage uniquement)
macos/Adapters (Swift : DiskArbitration/volume UUID, SMAppService/launchd, notifications, bookmarks/permissions)
i18n/        (fr.lproj, en.lproj ; codes → messages, pluriels via stringsdict)
```
Règles : aucune logique de copie/purge/restauration en Swift ; pas de dépendance circulaire ; pas de système de plugins ; extension = traits Rust ciblés (`Source`, `Destination`) introduits seulement quand un 2ᵉ cas réel existe (non créés en V1).

## 4. Historique, restaurabilité, vérification, purge
**Stockage** (destination `/<Dépôt>/`) :
- `objects/ab/cdef…` : un fichier = un objet immuable nommé par son hash (BLAKE3 proposé : rapide, crate Rust ; alternative SHA-256 accélérée matériellement — licence/maintenance à vérifier).
- `snapshots/<id>.manifest` : liste complète de l'arborescence (chemin, hash, taille, mtime, mode, xattrs/ACL pris en charge, liens) → **chaque point est un état complet restaurable**, même si peu d'objets sont nouveaux.
- `tmp/` : écritures en cours ; `journal/` ; `repo.json` (version de format, UUID du dépôt, identité du volume).
**Incrémental** : comparaison taille+mtime = *indice* ; si identiques et déjà indexés, objet réutilisé sans recopie ; vérification par hash configurable (rapide / complète). Ancien objet jamais modifié.
**Renommage/suppression source** : le nouveau manifeste reflète l'état source ; les anciens manifestes et objets restent. Une erreur de lecture ≠ suppression : le fichier est signalé « illisible », et on reprend la dernière version connue dans le manifeste (point marqué partiel).
**Validation d'un point** : tous objets du manifeste présents + hash vérifiés (échantillon ou total) → manifeste écrit en `.partial` puis renommé atomiquement `.manifest` (= réussi). Partiel/échoué : marqué tel quel, **ne remplace jamais** le pointeur « dernière réussite ».
**Reprise** : état de run persistant ; seuls les objets déjà écrits *et* vérifiés sont sautés ; verrou exclusif par dépôt (pas de run concurrent).
**Purge** : calcul d'un plan (versions, dates, nombre, espace estimé = objets uniquement référencés par les versions retirées) → **confirmation obligatoire** ; différée si absent ; interdit de retirer la dernière réussite ou de descendre sous 1 réussite ; suppression d'objets seulement par accessibilité (aucun manifeste conservé ne les référence), jamais pour faire de la place à un run non établi.
**États** : jamais exécutée, en attente, en cours, réussie, partielle, interrompue, échouée ; `dernière tentative` et `dernière réussite` stockées séparément.

## 5. Fidélité des fichiers (V1, à signaler dans le journal si perdue)
Liens symboliques : stockés comme lien (non suivis). Liens physiques : traités comme fichiers indépendants (dédup par hash). Permissions POSIX, mtime, xattrs : conservés si le FS destination les supporte, sinon avertissement. ACL, flags, fichiers clairsemés : à confirmer (probablement : avertissement, clairsemés restaurés pleins). Unicode : chemins stockés en octets d'origine + clé normalisée pour détecter collisions NFC/NFD/casse.

## 6. Sécurité des chemins
Résolution canonique (realpath, liens), refus source = destination ou imbriquées dans un sens ou l'autre, refus de `..`, identification disque par UUID de volume + marqueur `repo.json` ; disque absent → run « en attente/échec », jamais de dossier créé sur le point de montage. Mode dev : bac à sable `.local/sandbox`, simulation par défaut.

## 7. Modèle de données et migrations
JSON versionné (`schema_version`) pour config, `repo.json`, manifestes, journaux (événements = code stable + arguments). Migration : copie de sauvegarde `.bak`, écriture temp, rename, marqueur « migration en cours » reprenable. Format futur inconnu → refus explicite en lecture seule. Chaque version lit/restaure les formats antérieurs ; aucune promesse inverse.

## 8. Permissions & arrière-plan (selon Q4/Q5)
Recommandé : non sandboxé, Developer ID + notarisation, Accès complet au disque accordé par l'utilisateur (documenté, révocable dans Réglages Système), minimum de privilèges, **aucun root**. Agent launchd enregistré via SMAppService seulement si Q5=A ; sinon documenter « s'exécute uniquement si l'app est ouverte ». Mac endormi/éteint : run manqué → notification + rattrapage unique (clé d'idempotence par créneau). Purge jamais automatique.

## 9. Écrans
- **Mes sauvegardes** : liste de tâches (nom, état icône+texte, source → destination, disque disponible ou non, dernière tentative / dernière réussite, « Sauvegarder maintenant », progression + Annuler).
- **Nouvelle sauvegarde** : assistant 4 étapes (source, disque, quand, conserver combien) ; options avancées repliées.
- **Historique** : sélecteur de date, navigation arborescente, « Restaurer vers… » (nouvel emplacement par défaut ; original = double confirmation).
- **Réglages** : langue (Système/Français/English), notifications, journaux.
Accessibilité : navigation clavier, libellés VoiceOver, clair/sombre, état jamais par la couleur seule.

## 10. Tests (extraits critiques, vrai FS dans répertoires temporaires)
Copie initiale/incrémentale ; ajout/modif/renommage/suppression ; immuabilité des anciens objets (hash avant/après) ; restauration complète comparée octet à octet ; kill pendant copie et pendant finalisation puis reprise ; disque absent/plein/droits refusés ; fichier changeant ; accents, NFC/NFD, collisions de casse, liens ; chemins identiques/imbriqués ; source inchangée (arbre hashé avant/après) ; purge protégée ; migrations + format futur ; i18n (clés manquantes, pluriels) ; verrou concurrent. Validation d'une version = restauration réussie + test de non-destruction, pas seulement compilation.

## 11. Jalons V1
M0 squelette repo + CI locale + i18n + protocole · M1 scan + chemins sûrs · M2 dépôt/manifestes + backup complet · M3 incrémental + reprise + annulation · M4 vérification + restauration · M5 rétention/purge confirmée · M6 UI SwiftUI (tâches, historique) · M7 planification/événements disque · M8 docs fr/en, signature/notarisation, durcissement.
Feuille de route : V1.1 exclusions · V2 snapshots APFS, NAS/réseau · V3 chiffrement · clone amorçable : étude de faisabilité seulement.

## 12. Limites connues
Pas d'instantané cohérent sans snapshot APFS ; destination non-APFS = perte possible de métadonnées ; pas de chiffrement V1 ; dépôt lisible seulement via l'app (si Q2=A).
