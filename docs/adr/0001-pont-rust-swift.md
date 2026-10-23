# ADR 0001 — Pont Rust–Swift : processus helper local
Décision : binaire Rust embarqué, dialogue JSON-lines versionné (stdin/stdout), aucun serveur réseau.
Raisons : un crash moteur n'emporte pas l'UI ; annulation par message/signal ; testable sans Xcode.
Compromis : sérialisation, cycle de vie du processus à gérer côté Swift.
