# dsg-examples

Ensemble de réseaux dimensionnés comme exemples pour la simulation

## Contenu versionné (sous Git)

- ‘xxx.dsg’ : réseau dimensionné sous un format NW4 enrichi des sections !Clients et !Bornes. La section !Default properties est enrichie.
- ‘xxx.mat’ : fichier contenant toutes les tables générées par makeDesign.m
- ‘xxx.json’ : même chose stocké en format .json.

Les fichiers au format ‘xxx.dsg’ peuvent être lus par NW4. Les fichiers générés par NW4 doivent être enrichis. Utiliser le script /matlab/InOut/maj_NW4_NW5.m pour effectuer cette transformation.