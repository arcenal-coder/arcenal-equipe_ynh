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

Invocation Incus complète suivante : ÉCHEC d’environnement, résolution DNS
et accès réseau absents dans le conteneur. L’installation a refusé le
téléchargement des sources ; aucun cycle YunoHost n’est déclaré réussi.
Le runner autorise désormais explicitement le transfert du réseau incusbr0
et les scénarios root/subdir sont fixés pour cette application web.

L’environnement réseau est maintenant joignable. Le checker a révélé une
directive `only` non autorisée sur la suite default et une description de
paquet manquante. La suite par défaut est rétablie sans exclusion ; la détection
web du script install et la documentation sont corrigées.

Les tests web ont ensuite été arrêtés par l’absence de pycurl sur le host
du checker. Les cinq dépendances Python déclarées par l’outil amont sont
désormais disponibles dans l’environnement de vérification isolé.

Le checker a ensuite reçu HTTP 401 pour sa connexion SSO de test. Le mot de
passe des comptes factices du conteneur est aligné sur celui déclaré par
package_check avant installation ; cela ne concerne aucun compte de production.

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
