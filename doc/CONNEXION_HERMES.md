# Connexion du MCP d’Élise à Hermes

Le serveur MCP natif est exposé à `<origine><chemin>/mcp/elise`.
Pour une installation sous `/equipe`, utiliser par exemple
`https://exemple.fr/equipe/mcp/elise`.
L’espace Élise affiche l’adresse calculée à partir de la configuration YunoHost.

## Champs Hermes

L’API Hermes `POST /api/mcp/servers` accepte ces paramètres :

```json
{
  "name": "arcenal-elise",
  "url": "https://exemple.fr/equipe/mcp/elise",
  "profile": "elise",
  "auth": "header",
  "bearer_token": "<jeton créé dans ARCenal Équipe>"
}
```

## Parcours dans l’application

1. Ouvrir **Élise → Connexion MCP** dans ARCenal Équipe.
2. Copier l’adresse du serveur et cliquer **Créer un jeton pour Hermes**.
3. Dans Hermes, sélectionner le profil **elise**, ajouter un serveur MCP de
   transport **HTTP**, nommé **arcenal-elise**, et coller l’adresse.
4. Choisir l’authentification **Bearer token**, puis coller le jeton créé.
5. Enregistrer et utiliser **Test** dans Hermes pour découvrir les cinq outils.

Pour un ajout par API Hermes, utiliser le JSON ci-dessus. Les valeurs de
`auth` et `bearer_token` correspondent au formulaire Hermes réellement déployé.

Il s’agit d’un serveur HTTP Streamable stateless, sans commande locale ni
modification de Hermes. Le client envoie le jeton dans l’en-tête standard
`Authorization: Bearer …`. Les arguments des outils contiennent uniquement
les données métier : aucun `_arc_context`, identifiant LDAP ou secret.

## Droits et cycle de vie du jeton

- `initialize`, `ping`, `tools/list` et les notifications utilisent le même
  jeton Bearer que les appels d’outils ; aucun contexte propriétaire n’est requis.
- Les notifications JSON-RPC sans identifiant reçoivent HTTP 202 sans corps.
- Le jeton est créé depuis une session SSO YunoHost autorisée, pour un agent.
- L’application ne conserve que son empreinte SHA-256, jamais sa valeur brute.
- Il est affiché une seule fois, lors de la création ; le navigateur ne le
  conserve pas dans un stockage local. Hermes le stocke dans l’environnement
  protégé de son profil, selon son fonctionnement standard.
- Les appels sont attribués au propriétaire LDAP de cette connexion, et non à
  une identité choisie dans le message ou les arguments de l’outil.
- Chaque appel d’outil vérifie la validité du jeton, son agent, son expiration,
  sa révocation et les droits actuels de son propriétaire dans LDAP.
- Un jeton est valable 90 jours. Sa révocation est disponible dans
  **Mes connexions MCP** ; chaque utilisateur gère ses propres connexions.
- Création et révocation sont journalisées sans secret. La table des empreintes
  suit la sauvegarde/restauration PostgreSQL existante.
- Nginx transmet l’en-tête Authorization au service natif par son socket privé.
- Les erreurs d’authentification renvoient HTTP 401 avec `WWW-Authenticate`.
  Les erreurs métier sont des résultats MCP avec `isError: true`.
- La publication, suppression et l’envoi externe ne sont pas exposés comme outils.

## Périmètre de vérification

Le MCP s’utilise depuis Hermes, qui gère sa propre boucle d’outils. Le dialogue
textuel `llm.oneshot` intégré à ARCenal Équipe constitue un parcours distinct.
Il ne reçoit plus de contexte signé dans ses instructions.

La gestion des connexions et la correction HTTP 202 sont livrées dans
**0.2.0~ynh3**, publiée en preview. Mettre à jour l’application depuis ce canal
avant de suivre le parcours ci-dessus. La recette de connexion depuis Hermes
avec LDAP réel reste à effectuer sur l’installation client mise à jour.
