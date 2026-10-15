# Livraison MCP — préparation de 0.2.0~ynh3

Statut : correctif local vérifié, publication et déploiement non effectués.

## Version et publication proposées

- Commit fonctionnel : `feat: expose standard MCP connections for Hermes`.
- Binaires Linux : prérelease `v0.2.0-alpha.3`, amd64 et arm64.
- Paquet : `0.2.0~ynh3`, tag figé proposé `v0.2.0-ynh3`.
- Distribution : development puis preview après les contrôles applicables.

Le manifeste conserve les sources de la candidate précédente tant que les
nouveaux binaires ne sont pas publiés. Ne pas annoncer le nouveau paquet
installable avant remplacement de ses URL et SHA-256 par les valeurs réelles.

## Contenu

- MCP HTTP Streamable avec authentification Bearer standard.
- Onglet **Connexion MCP**, URL calculée, émission et révocation de jetons.
- Propriété LDAP, limitation à l’agent, expiration 90 jours, stockage des
  empreintes et audit sans secret.
- Schémas d’outils métier, notifications HTTP 202 sans corps et résultats
  d’erreur MCP ; aucun argument propriétaire d’authentification.
- Nginx transmet Authorization ; la permission publique du point d’entrée MCP
  laisse le service vérifier son propre jeton.
- Table PostgreSQL additive et idempotente, créée au démarrage du service.

## Preuves locales déjà obtenues

- Format, Clippy, compilation/types et build Rust réussis.
- 19 tests unitaires Rust et 1 test d’intégration réussis avec PostgreSQL 17.11
  isolé réel : mutations MCP, auteur, interdiction de publication, propriété,
  expiration, révocation et absence de secrets dans les listes/audits.
- Syntaxe JavaScript/Bash, parsing du manifeste et `git diff --check` réussis.
- 18 tests Python exécutés ; préparation de la classe DemoHTTPTests bloquée par
  l’occupation du port local 8080. Aucun processus préexistant arrêté.
- Formulaire Hermes inspecté en lecture : HTTP, Authentication **Bearer token**,
  champs API `auth=header` et `bearer_token` confirmés.

## Contrôles avant diffusion

1. Publication autorisée du commit fonctionnel, puis CI complète et construction
   des binaires Linux pour la même révision.
2. Mettre le manifeste à `0.2.0~ynh3` avec les nouvelles URL et empreintes réelles.
3. Vérifier le cycle YunoHost sur cette révision de paquet : root, sous-chemin,
   upgrade, sauvegarde/restauration, change_url et linter. Le contrôle d’écran
   SSO inclut le formulaire de création du jeton MCP.
4. Qualifier la même révision avec son propre run de cycle de vie et son entrée
   de catalogue development. Les preuves de 0.2.0~ynh2 ne qualifient pas ynh3.
5. Figer la version vérifiée et promouvoir l’entrée reçue en preview.

## Recette après mise à jour

- SSO : l’onglet MCP est visible dans Élise.
- Créer un jeton depuis une session LDAP autorisée.
- Ajouter URL et jeton au profil elise de Hermes ; le test liste cinq outils.
- Appeler `elise_identity` et vérifier le propriétaire LDAP de la connexion.
- Révoquer le jeton dans ARCenal Équipe : le nouvel appel Hermes est refusé.
- Aucun appel de génération de modèle n’est nécessaire pour cette recette MCP.

## Retour arrière

Avant la mise à jour réelle, conserver la sauvegarde YunoHost de l’application.
En cas d’échec, restaurer cette sauvegarde et la candidate précédente vérifiée
`v0.2.0-ynh2`. Elle ignore la nouvelle table et ne prend pas en charge les
connexions Bearer. Après restauration, vérifier SSO, données et fichiers ; les
jetons créés après la sauvegarde ne font pas partie de l’état restauré.
