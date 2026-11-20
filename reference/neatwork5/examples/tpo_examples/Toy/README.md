# Topologie Toy

Ensemble de **réseaux de topology Toy** utilisés pour tester NeatWork v5.

## Contenu versionné (sous Git)
- `DiamBaseToy.db` base de données des tuyaux (diamètres, coûts, etc.)
- `orifices.db` : base d’orifices (équipements terminaux).
- `xxx.tpo` : données topologiques d’un réseau **non dimensionné**.
-
Ces fichiers servent de **références d’entrée** pour lancer les pipelines. Tous les fichiers commencent par une section **!Commentaires** qui est ignorée à la lecture par le module createDesign ou par NW4. Cette section renseigne sur les particularités du modèle.

Les fichiers peuvent être lus et traités par le module **createDesign.m** et par **NW4** pour comparaison, à l'exception des exemples avec clients non-uniformes. Ces derniers ne peuvent être traités que par createDesign.m.

## Désignation des fichiers
Les fichiers sont sous le nom {Toy_}{x1}{x2}{x3}
- `x1` désigne la version de la topologie. x1 = b désigne le modèle de base.
- `x2` définit le mode de gestion de la borne soit débits bornés supérieurement (NW5) x2 = b, soit non bornés (NW4 et NW5), x2 = l.
- `x3`définit si les clients sont uniformes (NW4 et NW5) x3 = u ou différents (NW5) x3 = d.

`x2` et `x3` sont lus par NW5 dans `Default properties` et/ou `Clients` et `Borne`. Les informations de `Clients` et `Bornes` préemptent celles de `Default properties`.

- Exemple : `Toy_bbu`designe la version de base avec débits bornés et clients uniformes.

## Description des sections du fichier des modèles
- `!Commentaires` décrit les caractéristiques spécifiques du modèle (ignorée par NW4 à la lecture).
- ` Noeuds` (6 champs), `!Pipes` (3 champs) et 'Default properties` (2 champs) sont les sections habituelles de NW4 (donc lues par NW4).
- `!Clients` (4 champs), propre à Matlab, donne la borne à laquelle est rattaché le client, sa demande et sa probabilité de connexion au réseau (ignorée par NW4 à la lecture).
- `!Bornes` (3 champs) donne la liste des noeuds terminaux et la pression qui doit être assurée (ignorée par NW4 à la lecture). Le troisième champ ne contient que des 'N' qui ne sont pa utilisés par createDesign.

👉 La topologie étant simpliste, le comportement hydraulique est plus instable lorsque les probabilités d'ouverture sont faibles (paramètre `opentaps' dans '!Default properties'). Pour les comparaisons valables, il faut s'assurer que les paramètres de !Default properties' sont inchangés.
