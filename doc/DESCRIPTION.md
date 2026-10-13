ARCenal Équipe regroupe les assistants métier Élise, Victoire et Andréa dans
une application YunoHost avec authentification LDAP/SSO.

La candidate 0.2 fournit le recrutement natif dans PostgreSQL : candidatures,
pipeline, vivier, offres, réglages, CV PDF privés et journal d’activité. Les
publications et suppressions demandent une confirmation humaine. La page
publique ne présente que les offres publiées.

Un seul service Rust assure les pages et les trois façades MCP. La connexion
Hermes se configure côté serveur ; aucun secret n’est publié dans les profils
ou dans le navigateur. Les domaines complets Victoire et Andréa viennent
après réception du recrutement natif.
