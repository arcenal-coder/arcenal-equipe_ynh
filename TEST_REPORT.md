# Rapport de vérification — ARCenal Équipe 0.2

Statut : candidate native ; réception YunoHost en préparation.

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
