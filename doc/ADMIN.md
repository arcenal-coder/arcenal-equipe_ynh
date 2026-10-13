## Configuration et données

Le service utilise le socket privé `/run/__APP__/app.sock`, PostgreSQL et
le répertoire de données déclaré par YunoHost. Les comptes restent gérés
exclusivement par YunoHost ; l’application ne gère pas de mots de passe.

La configuration protégée se trouve dans `__INSTALL_DIR__/config/server.env`.
Configurer `HERMES_URL` (HTTPS pour une adresse externe) et `HERMES_TOKEN`
côté serveur. Ne jamais copier ces valeurs dans les YAML de profils.

Une sauvegarde YunoHost comprend l’application, la configuration, les profils,
les fichiers et PostgreSQL. La reprise initiale importe des métadonnées JSON
dans une base vide ; les CV se reprennent séparément avant réception de la bascule.
Ne pas supprimer l’ancienne application avant validation et sauvegarde de référence.
