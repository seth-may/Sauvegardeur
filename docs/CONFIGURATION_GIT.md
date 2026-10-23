# Configurer le dépôt Git

Cette procédure est à exécuter vous-même sur le Mac. Aucun dépôt local ou distant n’a été créé par la préparation des fichiers.

## 1. Vérifier Git et définir son identité

```bash
git --version
```

Si les outils de développement Apple ne sont pas installés :

```bash
xcode-select --install
```

Créer le dossier puis y placer le contenu du kit :

```bash
mkdir -p ~/Developer/projet-sauvegarde
cd ~/Developer/projet-sauvegarde
git init -b main
```

Configurer une identité propre à ce projet, en remplaçant les exemples :

```bash
git config user.name "Votre nom"
git config user.email "votre-adresse@example.com"
git config --get user.name
git config --get user.email
```

L’adresse apparaît dans les commits. Utiliser, si souhaité, l’adresse privée proposée par l’hébergeur. Ajouter `--global` uniquement si ces valeurs doivent s’appliquer à tous vos dépôts.

## 2. Vérifier les exclusions

Copier le `.gitignore` fourni à la racine avant d’ajouter des fichiers.

```bash
git status --short --untracked-files=all
git check-ignore config/config.local.json .local/sandbox/source/exemple.txt
```

Conserver dans Git : code, documentation, modèles, Cargo.lock de l’application, fichiers Xcode et schemes partagés. Exclure : vraies sauvegardes, données personnelles, clés, secrets, configuration locale et produits de compilation.

Un fichier déjà suivi n’est pas protégé par un nouvel ajout au .gitignore. Ne jamais publier un secret ; s’il l’a été, le révoquer et traiter l’historique, pas seulement le fichier actuel.

## 3. Premier commit

Après vérification manuelle des fichiers :

```bash
git add README.md .gitignore docs config/config.example.json PROMPT_FINAL.txt
git diff --cached --stat
git diff --cached
git commit -m "docs: cadrage initial du projet de sauvegarde"
```

Adapter cette liste si certains documents n’ont pas encore été copiés.

## 4. Créer le dépôt distant

Sur GitHub ou un autre hébergeur :

1. Créer un dépôt privé nommé par exemple projet-sauvegarde.
2. Le laisser vide : pas de README, licence ou .gitignore généré à distance.
3. Choisir une méthode d’authentification prise en charge : HTTPS avec gestionnaire d’identifiants, ou SSH déjà configuré.
4. Copier l’URL exacte proposée par l’hébergeur.

Pour GitHub, remplacer VOTRE_COMPTE et NOM_DU_DEPOT dans cet exemple :

```bash
git remote add origin https://github.com/VOTRE_COMPTE/NOM_DU_DEPOT.git
git remote -v
git push -u origin main
```

L’URL ci-dessus est un modèle, pas un dépôt créé. Ne pas inclure de jeton dans l’URL. Si origin existe déjà, examiner sa valeur avant toute modification :

```bash
git remote -v
# Uniquement si le changement est voulu :
git remote set-url origin https://github.com/VOTRE_COMPTE/NOM_DU_DEPOT.git
```

Si le distant contient déjà des commits, ne pas forcer l’envoi : cloner d’abord le dépôt dans un nouveau dossier et y intégrer les fichiers après comparaison.

## 5. Travail quotidien avec une IA

```bash
git status
git switch -c feat/premier-moteur
# Faire les modifications, relire les différences et exécuter les tests.
git diff
# Ajouter explicitement les fichiers souhaités :
git add README.md
git diff --cached
git commit -m "docs: préciser le périmètre du moteur"
git push -u origin feat/premier-moteur
```

L’ajout de README.md est un exemple : adapter la liste aux vrais changements. Utiliser une branche par évolution, des commits courts et une revue avant fusion. Ne pas accepter de commandes destructrices proposées automatiquement par une IA. Ne pas utiliser `git reset --hard`, `git clean -fd` ou un push forcé sans comprendre leurs conséquences.

## 6. Jalons

Après validation réelle d’une version :

```bash
git tag -a v0.1.0 -m "Premier jalon validé"
git push origin v0.1.0
```

Un dépôt distant protège le code versionné, pas les données utilisateur ni les fichiers ignorés. Prévoir une sauvegarde indépendante du poste de développement.

## Sources techniques

- Git init : https://git-scm.com/docs/git-init
- Gestion des remotes : https://git-scm.com/docs/git-remote
- GitHub : https://docs.github.com/en/get-started/git-basics/managing-remote-repositories
