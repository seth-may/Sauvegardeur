# Journal des versions (pré-V1)
- M0 : workspace, types, i18n fr/en, protocole helper (hello/ping), ADR 0001-0002.
- M1 : fs-scan — chemins sûrs, parcours lecture seule, collisions Unicode/casse.
- M2 : store — dépôt d'objets BLAKE3, manifestes, sauvegarde, restauration.
- M3 : verrou, annulation, reprise, tests SIGKILL, protocole backup/cancel.
- M4 : vérification rapide/complète, restore/list via helper, catalogue i18n complet.
- M5 : retention — plan/exécution de purge confirmée.
- M5bis : archive complète (code + kit de documentation).
Prochain : persistance des tâches + config JSON versionnée, puis M6 (SwiftUI), M7 (launchd, disques), M8 (docs, signature).
