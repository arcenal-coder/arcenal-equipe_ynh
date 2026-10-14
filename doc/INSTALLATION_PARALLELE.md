# Installation parallèle — ARCenal Équipe 0.2.0~ynh2

## Destination

- Paquet distinct : `arcenal_equipe`.
- Nom : ARCenal Équipe.
- Chemin proposé : `/equipe`, sur le domaine retenu pendant l’installation.
- Service, base PostgreSQL et répertoire de données propres au paquet.
- Source : https://github.com/arcenal-coder/arcenal-equipe_ynh

L’installation n’importe pas de données automatiquement et n’écrit pas dans
l’application actuelle. La reprise initiale se fait après installation et
avant toute décision de bascule. Les deux applications peuvent coexister.

## Parcours client YunoHost

1. Ouvrir **Applications → Installer une application → Installer une application personnalisée**.
2. Utiliser l’URL GitHub ci-dessus et la branche `main` ou la révision reçue.
3. Choisir le domaine, le chemin `/equipe` et le groupe LDAP autorisé.
4. Ouvrir ARCenal Équipe depuis le portail YunoHost.

Le référencement Store en prévisualisation permettra ensuite une installation
depuis le catalogue. La présence du code GitHub ne signifie pas que cette entrée
de catalogue a déjà été publiée.

## Recette commune avant bascule

- Connexion SSO, refus d’un compte non autorisé et prénom LDAP.
- Candidature de test : création, modification, pipeline, vivier, CV PDF et audit.
- Offre de test : brouillon, aperçu/confirmation, publication, archivage et page publique.
- Réglages et modèles de messages ; confirmations avant les actions critiques.
- Sauvegarde, restauration, mise à jour et changement de chemin.
- Connexion au moteur Hermes ; ne pas confondre accès API et réponse effective du modèle.
- Reprise des métadonnées puis des pièces jointes ; rapprochement des comptages
  et vérification des documents avant réception de la migration.

La suppression de l’ancienne application est une décision séparée après cette
recette, une sauvegarde de référence et validation explicite du client.

## Périmètre de la candidate

Le recrutement est natif. Les domaines complets de Victoire et Andréa et la
boucle de dialogue MCP restent à recevoir selon le cadrage. Cette version est
destinée à l’installation parallèle et à la recette ; elle ne déclare pas la
V1 complète réceptionnée.
