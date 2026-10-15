# Installation et mise à jour — ARCenal Équipe 0.2.0~ynh3

## Destination

- Paquet distinct : `arcenal_equipe`.
- Nom : ARCenal Équipe.
- Chemin proposé : `/equipe`, sur le domaine retenu pendant l’installation.
- Service, base PostgreSQL et répertoire de données propres au paquet.
- Source : https://github.com/arcenal-coder/arcenal-equipe_ynh
- URL figée pour installation : https://github.com/arcenal-coder/arcenal-equipe_ynh/tree/v0.2.0-ynh3
- Révision vérifiée : `09bc15c42a193463856a937975f1f5a48d13fb6b`.
- Cycle YunoHost 12.1.41.2 : root, subdir, upgrade, backup/restore et change_url réussis.

L’installation n’importe pas de données automatiquement et n’écrit pas dans
l’application actuelle. La reprise initiale se fait après installation et
avant toute décision de bascule. Les deux applications peuvent coexister.

## Parcours client YunoHost

1. Ouvrir **Applications → Installer une application → Installer une application personnalisée**.
2. Utiliser l’URL figée GitHub ci-dessus pour installer exactement la révision vérifiée.
3. Choisir le domaine, le chemin `/equipe` et le groupe LDAP autorisé.
4. Ouvrir ARCenal Équipe depuis le portail YunoHost.

## Depuis ARCenal Store

La version 0.2.0~ynh3 est publiée dans le catalogue **Prévisualisation**.

1. Dans la configuration d’ARCenal Store, choisir **Prévisualisation**, puis
   **Actualiser le catalogue**.
2. Dans **Applications → Installer une application**, rechercher **ARCenal Équipe**.
3. Installer dans un chemin distinct, par exemple `/equipe`.

La qualification technique du catalogue ARCenal est **5**. Elle est calculée
sur les tests du paquet et son enregistrement dans le catalogue réellement
utilisé. La note du catalogue communautaire officiel reste distincte et est
conservée dans `QUALIFICATION_ARCENAL.json`. Ce n’est pas une certification officielle.

## Mettre à jour une installation existante

1. Vérifier qu’une sauvegarde YunoHost récente d’ARCenal Équipe est disponible.
2. Dans ARCenal Store, sélectionner **Prévisualisation** puis actualiser le catalogue.
3. Mettre à jour l’application **ARCenal Équipe** vers **0.2.0~ynh3** depuis YunoHost.
4. Ouvrir **Élise → Connexion MCP** pour copier l’URL et créer le jeton à renseigner dans Hermes.

Le service crée la nouvelle table de connexions MCP au démarrage ; la migration
est additive. La publication du paquet dans le catalogue ne met pas à jour
automatiquement une application installée.

Voir [la procédure Hermes](CONNEXION_HERMES.md) et [le retour arrière](LIVRAISON_MCP.md).

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

Le recrutement et le serveur MCP sont natifs. L’ajout et l’appel d’outil depuis
Hermes restent à recevoir sur l’installation client mise à jour. Les domaines
complets de Victoire et Andréa suivent la réception du recrutement selon le
cadrage. Cette version est destinée à l’installation parallèle et à la recette ;
elle ne déclare pas la V1 complète réceptionnée.
