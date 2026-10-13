# ARCenal Équipe — premier POC

## Version native 0.2 — livraison en préparation

La commande de production est `arcenal-equipe serve` : un service Rust/Axum,
une base PostgreSQL, authentification exclusivement par le proxy SSO YunoHost
et lecture du prénom dans LDAP. Aucun compte ni mot de passe applicatif.

**Dépôt de livraison :** https://github.com/arcenal-coder/arcenal-equipe_ynh

Le recrutement est natif : création/modification des candidatures et offres,
pipeline, vivier, archivage/suppression confirmés, réglages administrateur,
CV PDF privés, journal d’audit LDAP, page publique des offres publiées.
Les confirmations de publication sont liées à l’auteur, au contenu et à la
version du dossier ; une confirmation expirée, réutilisée ou modifiée est refusée.

La reprise initiale importe les métadonnées dans une base vide, en transaction,
préserve les identifiants et les dates source et demande une confirmation de
la nouvelle destination publique. Les pièces jointes se reprennent séparément.
La bascule et la suppression de l’application précédente restent soumises à réception.

Les façades MCP vérifient un contexte signé par ARCenal, son expiration, son
agent et l’utilisateur LDAP avant chaque action. Aucun auteur libre n’est accepté.
Les pages privées exigent l’en-tête SSO du proxy attendu et un secret de proxy
injecté côté Nginx ; ce secret ne paraît jamais dans le navigateur.

**Statut :** service compilé et contrôles unitaires locaux exécutés ; construction
Linux et essais PostgreSQL/YunoHost en cours de préparation sur GitHub. Aucune
installation YunoHost ni promotion Store n’est déclarée réussie à ce stade.
Victoire et Andréa restent visibles mais leurs domaines complets viennent après
la réception du recrutement natif. La conversation utilise pour l’instant une
complétion textuelle Hermes ; le dialogue avec boucle MCP native reste à recevoir.

Les sections suivantes documentent le pilote de développement antérieur.

## POC connecté à l’instance réelle

Le mode `--connected` affiche l’identité réellement vérifiée par YunoHost et
utilise le profil Hermes **`elise` existant**, sa description et son modèle.
Le thème reprend la palette **Gratitude — Système** lue sur votre instance.
La conversation dispose d’un formulaire, d’un historique court en mémoire,
d’états d’attente/erreur et d’un bouton de vérification de connexion.

### Espace métier d’Élise

L’espace s’ouvre désormais sur **Candidatures**, avec six colonnes de pipeline.
Les onglets **Vivier**, **Offres**, **Réglages** et **Activité** reprennent les
données réelles de l’application ONYX ; **Conversation** reste disponible à côté.

- Pipeline : recherche, filtre de poste, fiches candidat et étapes existantes.
- Vivier : recherche, filtre de métier, compteurs et fiches candidat.
- Offres : liste, statuts, détails et affichage des archives.
- Réglages : connecteurs, modèles de non-retenue/newsletter et École d’Élise.
- Activité : journal avec auteur, date et action.

Ces cinq vues sont intégrées **en consultation**. Les liens de gestion ouvrent
l’application ONYX existante pour créer, modifier ou enregistrer. Le relais
utilise exclusivement un GET fixe `api-v2.php?action=bootstrap`, avec la même
identité SSO vérifiée. Il ne transmet aucun ordre de mutation.
Les références de secrets, URL de connecteurs et chemins serveur des CV sont
retirés avant l’envoi au navigateur ; aucune donnée candidat n’est envoyée
automatiquement à Hermes. Les données ne sont pas recopiées dans une autre base.

Ce relais est transitoire pour le POC ; il ne remplace pas la cible Rust et
PostgreSQL uniques. Les mutations natives et la migration restent à réaliser.

**Dernier contrôle réel :** SSO et transport Hermes fonctionnent ; les trois
essais de génération ont échoué (délai dépassé ou Gemini HTTP 503, forte
demande). Aucune réponse de conversation réussie n’est donc déclarée. Les
messages sont conservés dans le formulaire pour un nouvel essai ; aucune
réponse de secours ou simulée ne remplace l’erreur.

Le POC connecté est lancé sur **http://127.0.0.1:8080/**. C’est un pilote local
mono-utilisateur : la session de processus est celle validée par YunoHost.
Le mode YunoHost multi-utilisateur reste distinct et doit être éprouvé sur
l’environnement de test avant installation.

### Redémarrage après expiration de la session

Avec les outils déjà présents (Rust et Python 3), préparer une nouvelle session
depuis le terminal ; le mot de passe est saisi de façon masquée et n’est pas
enregistré. Le répertoire parent externe doit déjà exister :

```sh
python3 -B tools/connect_preview.py --runtime-parent /chemin/vers/un/repertoire/prive
ARCENAL_SESSION_FILE=/chemin/retourne/session.json cargo run --offline -- --connected
```

La préparation authentifie directement auprès du SSO YunoHost ; elle ne crée
ni compte applicatif ni nouveau profil Hermes. Les cookies et le jeton de
session restent dans un répertoire temporaire privé (0700), fichier 0600,
hors dépôt. Ils ne sont envoyés ni dans l’HTML ni dans le JavaScript local.
Une session SSO révoquée ou une identité différente refuse la génération.

### Transport et périmètre de ce pilote

- Rust reste le seul service applicatif. `lib/hermes_bridge.py` est un client
  HTTPS/WebSocket ponctuel, utilisant uniquement la bibliothèque standard.
- Le SSO est revalidé avant chaque message. L’identifiant complet sert à
  l’attribution. Le pilote distant reçoit le nom d’affichage YunoHost ;
  `givenName` réel reste à vérifier par le lecteur LDAP en mode YunoHost.
- Transport vérifié : session Hermes temporaire sur `elise`, puis RPC
  `llm.oneshot` empruntant son runtime. La description de profil existante est
  conservée ; aucun changement de modèle ou de configuration distant.
- Les réponses sont textuelles, sans accès aux candidatures ni outil de
  mutation ; l’historique court reste en mémoire du service local.
- Le serveur exécute son outil MCP d’identité local et transmet ce résultat
  dans le contexte. **Ce n’est pas un serveur MCP enregistré dans Hermes.**
  Le raccordement MCP natif et le contexte de session vérifiable de bout en
  bout restent à réaliser avant réception du socle complet.
- Élise seule est raccordée dans ce lot. Les autres agents restent visibles
  avec un état explicite, sans conversation simulée.

Contrôles effectués : 11 tests Rust et 18 tests Python réussis ; navigateur
réel à 1440 et 360 pixels sans débordement, accès clavier au bouton d’envoi,
thèmes clair/sombre, cinq onglets métier, recherche, ouverture/fermeture clavier
d’une fiche candidat et vérification SSO/Hermes. Le dialogue réel est testé
mais **bloqué côté modèle** ; l’installation YunoHost n’est pas validée.

## Mode initial sans connexion

Un service Rust, trois espaces d’agents, trois façades MCP de démonstration.
Ce POC est en lecture seule : aucune donnée métier, aucun compte applicatif,
aucune publication. Aucun paquet Rust externe n’est nécessaire.

## Essayer localement

Avec Rust déjà installé :

```sh
cargo run --offline -- --demo
```

Ouvrir **http://127.0.0.1:8080/**. Camille est une identité fictive fixe,
indépendante des en-têtes du navigateur. Le bandeau indique ce mode.

1. Vérifier l’accueil avec Élise, Victoire et Andréa.
2. Ouvrir un espace et cliquer **Tester la connexion MCP**.
3. Lire le résultat : agent, prénom et auteur `demo.local`.
4. Télécharger le profil YAML, changer le thème, essayer au clavier et sur mobile.

Le test vérifie `initialize`, `tools/list` et `tools/call` du service local.
Il **ne vérifie pas Hermes**. La conversation présentée est une phrase de
démonstration, pas une réponse d’un modèle.

## Contrôles locaux

```sh
cargo fmt -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo check --offline --all-targets
cargo test --offline
cargo build --offline
python3 -B -m unittest discover -s tests -v
```

Les tests couvrent notamment les refus sans identité, l’isolation des outils,
l’usurpation via arguments, les origines intersites, les limites HTTP/JSON,
l’échappement HTML, le chemin d’installation et les replis du prénom LDAP.

## Mode YunoHost : candidate à tester

Le service sans `--demo` écoute uniquement sur un socket Unix privé.
Nginx relaie l’identité `YNH_USER` fournie par SSOwat, avec `auth_header = true`.
SSOwat nettoie les en-têtes d’identité fournis par le navigateur et vérifie la
permission de l’application. Le mot de passe YunoHost n’est ni transmis au
service ni conservé. Le socket est accessible au service et au groupe du
frontal `www-data`, pas aux utilisateurs locaux ordinaires.

`lib/identity.py` interroge le LDAP local YunoHost : `givenName`, puis
`displayName`, puis `cn`, puis identifiant complet. Une erreur LDAP refuse
l’accès ; un auteur n’est jamais remplacé par son prénom. L’accès anonyme aux
attributs et les en-têtes doivent être confirmés sur la version YunoHost cible.
Le programme Python court est un lecteur d’annuaire, pas un second service.

Le manifeste v2 déclare un utilisateur système, les permissions privées, les
répertoires et une seule base PostgreSQL. Cette base est réservée au futur
socle persistant : le POC ne stocke rien. La page recrutement publique n’est
pas encore exposée. Les ressources APT déclarées ne sont installées que lors
d’une installation YunoHost expressément autorisée, jamais par les tests locaux.

Les scripts `install`, `upgrade`, `remove`, `backup`, `restore` et les modèles
Nginx/systemd sont fournis comme **candidate non éprouvée sur YunoHost**.
Sauvegarde : binaire, configuration serveur, profils, données et dump PostgreSQL.
Mise à jour : configuration protégée conservée. Retour arrière : restauration
d’une sauvegarde YunoHost préalable sur la même architecture.

Le Mac produit un binaire macOS qui ne peut pas être installé sur YunoHost.
Sur un environnement Linux de construction autorisé, avec Rust disponible :

```sh
cargo build --release --offline
python3 tools/package.py --binary target/release/arcenal-equipe --output /tmp/arcenal-equipe-poc.tar.gz
```

Le générateur refuse un binaire macOS et un fichier de sortie déjà existant.
Aucun téléchargement ni déploiement n’est réalisé. La licence et les
mainteneurs restent à renseigner avant publication ; aucune licence n’a été
choisie à la place du propriétaire.

## Hermes : contrat à confirmer

Les profils de `profiles/` sont des YAML **indicatifs**, sans URL ni secret.
Leur compatibilité d’import doit être vérifiée avec la version réelle de
Hermes. Façades : `/mcp/elise`, `/mcp/victoire`, `/mcp/andrea`, préfixées par le
chemin d’installation éventuel. Transport POC : POST JSON-RPC avec réponse
JSON, sans SSE ni sessions persistantes ; `GET` MCP n’est pas supporté.

Chaque façade expose seulement `<agent>_demo_identity`. Aucun argument
d’identité n’est accepté. La réponse reflète exclusivement l’identité serveur.
En mode YunoHost, un client MCP doit lui aussi passer une identité SSO validée.
Hermes ne peut pas agir avec une identité inventée ou un simple identifiant
dans le prompt. **Le transport vérifiable des sessions entre ARCenal et Hermes
reste à concevoir/tester avec l’API réelle** ; aucun contournement SSO ou jeton
global ne lui donne accès dans cette version.

## Recette restante du premier lot

- YunoHost de test : installation, accès anonyme redirigé, utilisateur refusé,
  utilisateur autorisé, véritable `givenName` et refus d’en-têtes usurpés.
- Cycle de vie : mise à jour, sauvegarde, restauration, suppression.
- Hermes : version/API identifiées, import d’un profil et appel MCP attribué
  au bon utilisateur via un contexte serveur vérifiable.
- Interface : revue visuelle, clavier, mobile et audit WCAG ; thème et avatars
  provisoires à comparer aux éléments Gratitude déjà validés (non fournis).

## Lots suivants

1. Valider ce socle en environnement YunoHost/Hermes de test.
2. Élise : offres et recrutement public, avec confirmation et audit.
3. Victoire : brouillons documentaires et exports.
4. Andréa : contenus, calendrier et intégration recrutement.
5. Profils définitifs, recette complète, sauvegarde/restauration et livraison.

Les fonctions métier commencent après réception du socle.
