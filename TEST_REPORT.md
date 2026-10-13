# Rapport de vérification — ARCenal Équipe 0.2

Statut : candidate native ; réception YunoHost en préparation.

Première invocation du checker : NON_EXÉCUTÉ, arguments de README obsolètes
ayant affiché l’aide avec code retour 0. Le workflow est corrigé pour employer
les variables attendues et exige désormais un rapport JSON réel.

Deuxième invocation : NON_EXÉCUTÉ, préflight ICMP bloqué sur le runner alors
que HTTPS fonctionne. Le préflight accepte maintenant le téléchargement HTTPS
vérifié de l’index YunoHost ; aucun scénario de paquet n’est exclu.

Troisième invocation : NON_EXÉCUTÉ, le runner ne disposait pas du module TOML
et de l’alias local de l’image appci. La préparation installe le module et
importe désormais l’image officielle publiée dans le dépôt Incus YunoHost.

| Contrôle | Résultat | Preuve |
|---|---|---|
| Linter Rust, syntaxe JS/Bash, compilation | RÉUSSI | GitHub Actions `Vérifier le service natif` |
| Tests unitaires et protections de contexte signé | RÉUSSI | Tests Rust |
| Transactions PostgreSQL, confirmations, rejeu, conflit et audit UID LDAP | RÉUSSI | PostgreSQL 15 réel dans GitHub Actions |
| Construction Linux amd64 et arm64 | RÉUSSI | Artifacts des deux architectures |
| Secret scanning avant commit | RÉUSSI | Hook gitleaks, aucun secret détecté |
| Installation YunoHost | NON_EXÉCUTÉ | Workflow Incus à lancer après sources Linux figées |
| Retrait/réinstallation YunoHost | NON_EXÉCUTÉ | Même workflow |
| Upgrade YunoHost | NON_EXÉCUTÉ | Même workflow |
| Backup/restore données et configuration | NON_EXÉCUTÉ | Même workflow |
| Permissions SSO, prénom LDAP, accès web réel | NON_EXÉCUTÉ | Recette du paquet dans Incus |
| Reprise complète des CV et bascule depuis l’ancienne application | NON_EXÉCUTÉ | Métadonnées importables, pièces jointes à reprendre |
| Conversation réelle et boucle MCP Hermes | ÉCHEC | Trois essais sur l’instance fournie : délai/503 ; transport textuel natif et façades présents, boucle complète non reçue |
| Domaines Victoire et Andréa | NON_EXÉCUTÉ | Après réception du recrutement natif, suivant le cadrage |

Le dépôt et une construction Linux ne valent pas une installation ni une promotion Store.
