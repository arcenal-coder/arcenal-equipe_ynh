# Rapport de vérification — ARCenal Équipe 0.2

Statut : **candidate MCP 0.2.0~ynh3 vérifiée sur YunoHost 12.1.41.2** ; recette de connexion Hermes sur l’installation client à effectuer.

## Candidate MCP standard 0.2.0~ynh3

- Révision figée : `09bc15c42a193463856a937975f1f5a48d13fb6b`, binaires **alpha.3** amd64/arm64, SHA-256 vérifiés par téléchargement.
- CI native et PostgreSQL 15 réel : [38757866925](https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38757866925), réussie. 19 tests Rust unitaires et un test d’intégration ; protocole MCP, propriété LDAP des connexions, mutations attribuées, brouillon seul, expiration/révocation et stockage sans jeton brut.
- Construction Linux : [38757877929](https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38757877929), réussie sur les deux architectures.
- Cycle YunoHost : [38758390074](https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38758390074), six scénarios réussis : root, sous-chemin, upgrade, sauvegarde/restauration, change_url et linter.
- Qualification technique ARCenal **5**, résultat officiel brut **0** conservé : [38760615395](https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38760615395). Aucun échec technique ignoré.
- Formulaire Hermes réellement inspecté : HTTP et Bearer token standard (`auth=header`). Ajout et appel depuis Hermes après mise à jour client : **NON_EXÉCUTÉ**.
- Documentation de connexion : `doc/CONNEXION_HERMES.md` ; livraison/rollback : `doc/LIVRAISON_MCP.md`.

## Historique de la candidate 0.2.0~ynh2

Candidate parallèle précédente : **0.2.0~ynh2**, binaires **alpha.2** avec correction
du routage de sous-chemin. Révision testée : `2a75092f96606f4ef969d123615a9384d8084919`.
Preuve : https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38531575142
Les six scénarios du rapport package_check sont réussis ; le classement global
de l’outil reste à 0 (indicateur distinct, avec trois avertissements de linter).

## Qualification du catalogue cible corrigée

Le niveau 0 amont provenait de l’absence du paquet dans le catalogue communautaire
officiel, distinct du catalogue tiers ARCenal qui distribue ce paquet. Le workflow
`Qualifier pour le catalogue ARCenal` vérifie les six scénarios, refuse toute
erreur technique et compare la version/révision à l’entrée effectivement publiée
dans ARCenal Store. Résultat : **qualification technique ARCenal 5**, preuve
https://github.com/arcenal-coder/arcenal-equipe_ynh/actions/runs/38543953375.

Le résultat officiel brut n’est pas réécrit. Seules les deux constatations
strictement liées au catalogue officiel sont vérifiées contre le catalogue cible ;
aucune erreur de code, permission, sauvegarde ou restauration ne peut être ignorée.

Régression d’affichage sur le client : catalogue API niveau 5/état working,
carte visible avec le filtre normal « qualité satisfaisante », fiche ouverte
dans Chrome sans le libellé cassée. L’actualisation n’a concerné que l’index
des applications, sans installation ou mise à jour d’une application.

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

Rapport e0d625e sur YunoHost 12.1.41.2 : installation root et upgrade RÉUSSIS.
Sous-chemin, backup/restore et changement d’URL en ÉCHEC HTTP 404 pour `/path/`.
Cause identifiée : Axum 0.8 distingue un préfixe avec slash final. Le montage
du routeur est corrigé et un test couvre l’accueil et l’API en sous-chemin.

La construction de l’alpha.2 a rencontré un blocage réseau vers gnu.org,
après compilation réussie. La copie complète de GPL est désormais téléchargée
depuis le miroir SPDX GitHub avec délais et retries bornés.

| Contrôle | Résultat | Preuve |
|---|---|---|
| Linter Rust, syntaxe JS/Bash, compilation | RÉUSSI | GitHub Actions `Vérifier le service natif` |
| Tests unitaires et protections de contexte signé | RÉUSSI | Tests Rust |
| Transactions PostgreSQL, confirmations, rejeu, conflit et audit UID LDAP | RÉUSSI | PostgreSQL 15 réel dans GitHub Actions |
| Construction Linux amd64 et arm64 | RÉUSSI | Artifacts des deux architectures |
| Secret scanning avant commit | RÉUSSI | Hook gitleaks, aucun secret détecté |
| Installation YunoHost root et subdir | RÉUSSI | package_check, run 38531575142 |
| Retrait/réinstallation YunoHost | RÉUSSI | Cycle d’installation du checker |
| Upgrade YunoHost | RÉUSSI | package_check, run 38531575142 |
| Backup/restore du paquet et configuration | RÉUSSI | package_check, run 38531575142 ; données métier de production non migrées |
| Accès web SSO de test et ressources | RÉUSSI | curl_tests du paquet dans Incus ; recette LDAP client à effectuer |
| Changement d’URL | RÉUSSI | package_check, run 38531575142 |
| Reprise complète des CV et bascule depuis l’ancienne application | NON_EXÉCUTÉ | Métadonnées importables, pièces jointes à reprendre |
| Conversation réelle et boucle MCP Hermes | ÉCHEC | Trois essais sur l’instance fournie : délai/503 ; transport textuel natif et façades présents, boucle complète non reçue |
| Domaines Victoire et Andréa | NON_EXÉCUTÉ | Après réception du recrutement natif, suivant le cadrage |

Le dépôt et une construction Linux ne valent pas une installation ni une promotion Store.
