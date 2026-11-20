---
title: "Format des données dans NW5_Matlab"
author: "JPV, 19 mai 2026"
# date: '\today'

pdf-engine: xelatex

mainfont: "Arial"
fontsize: 12pt
geometry:
  - margin=2cm
  - a4paper
numbersections: true
# toc: true
# toc-depth: 2
header-includes:
  - \usepackage{setspace}
  - \setstretch{1.1}
  - \usepackage{fancyhdr}
  - \pagestyle{fancy}
  - \fancyfoot[C]{\thepage}
  - \fancyhead{}
  s- \renewcommand{\headrulewidth}{0pt}
  # pandoc FormatPourNW5.md -o FormatPourNW5.pdf --pdf-engine=xelatex
  
---
# Formats internes et externes pour NW5

***Le but premier du programme que j'ai développé en Matlab a été de créer deux moteurs d'optimisation, l'un pour la conception, l'autre pour la simulation.*** Afin de permettre la comparaison et la validation par rapport à NW4 et d'autres développement, j'ai assuré la compatibilité de lecture des fichiers utilisés et produits par NW4. Cette deuxième exigence a entraîné un gros travail de lecture et de création de fichiers de format .txt sous divers noms.

## Contribution algorithmique
Ma contribution essentielle concerne le développement et la validation de deux modèles d'optimisation. L'un est traité par **programmation linéaire** pour laquelle il existe des codes en Python, qui sont puissants et en source libre. L'autre par une méthode que j'ai développée, dite  **méthode de points intérieurs (IPM)**, pour programmation convexe  non-linéaire. Le modèle linéaire pour la conception est une *variante de celui développé dans NW4*, enrichi sur plusieurs points. Il doit être complété, voire remplacé, par un modèle basé sur l'optimisation robuste qui a été programmé en AMPL par Frédéric.

Le travail de Sarah porte en priorité sur la création de ces modèles et sur les méthodes de résolution. **Le produit doit être pérenne**. Le travail de lecture et export des données est malheureusement nécessaire, mais risque d'être partiellement obsolète après les interventions de Shahin.

## Structures de données internes
C'est la partie cruciale du programme Matlab.

-  Quelles sont les données requises pour faire fonctionner le programme de conception (design) et de simulation ? Quel format adopter pour les données pour créer les modèles mathématiques de conception et de simulation ?

- Comment communiquer les résultats de la conception au module de simulation ?
- Comment tenir compte des exigences posées par de nouvelles modélisation (comme l'irrigation) et rester ouvert à de possible extension de modélisation (par exemple reproduire des analyses de même nature que ce que propose EPANET) ?

### Choix des formats internes
Il m'est apparu que le modèle de conception et surtout celui de simulation sont axés sur les débits et que les débits sont rattachés aux arcs du réseau. Les données requises pour créer les modèles sont presque toutes rattachées aux arcs (borne sur les débits, coefficients de friction, et même force de gravité pour les arcs terminaux). J'ai donc créé une structure principale sous forme de table (TA pour les arcs). D'autres structures, sous forme de tables, ont dû être créées, TC pour les clients (demande, probabilité et borne de rattachement), TB pour les bornes (pression garantie), TN pour les nœuds (coordonnées spatiales tridimensionnelles, équipements, …), TD pour les bases de données des tuyaux disponibles., etc. J'utilise aussi une structure pour les paramètres techniques (comme les constantes hydrauliques) et les paramètre de design (comme la longueur commerciale des tuyaux, …). Les paramètres sont lus dans un premier temps dans les fichiers d'entrée (.tpo,.dsg) et transférés dans une table TP. 

Ainsi la premère partie du travail consiste à lire les données où elles se trouvent et à les introduire dans les structures internes : Les tables TA, TB, TC, TN, TP, TO (je cite de mémoire), la structure param et la structure dat. Cette dernière contient le nom du projet, l'adresse du dossier etc.

### Transmission des données de la conception à la simulation
A l'issue du modèle de conception on dispose des mêmes tables TA, TN, etc. qu'à l'entrée mais enrichies par les résultats de l'optimisation. Ainsi pour chaque arc (ligne du tableau TA) on a le schéma de découpe en un ou deux tronçons avec l'index du tuyau utilisé, sa longueuur et son diamètre. Dans la table TN à chaque ligne correspondant à un arc terminal on ajoute les orifices commerciaux et idéeaux (dans le cas NW4). Le **plus simple dans le cadre Matlab** est alors de stocker toutes ces éléments dans un **fichier .mat**. 

Ce mode de stockage des données concernant un réseau dimensionné ne suffit pas. En effet, il faut pourvoir transmettre les informations à d'autres outils, comme NW4. Il faut sussi pouvoir lire des fichiers en entrée de la simulation qui ne proviennent pas du code Matlab, par exemple des dimensionnement produits par NW4, voire EPANET ou autres.

## Stuctures externes et I/O
Le premier souci a été la compatibilité avec les formats .tpo et .dsg de NW4. Ces fichiers de format .txt sont organisés par sections (une section commence par un point d'exclamation, par ex. !Pipes) et les données par section sont organisées en lignes avec une virgule ',' comme séparateur. Je me suis aperçu que l'on pouvait ajouter des sections sans perturber la lecture par NW4. Dans les fichiers .tpo de NW4 on trouve des champs de certaines sections qui sont remplis par des 'N'. En Matlab cela aurait pu être de 'NaN'. Ces valeurs sont remplacées par des chiffres ou des mots dans les fichiers .dsg. Je pense qu'une meilleure solution aurait été de renseigner après la section, disons !Nodes, le nom des variables : 'ID', 'hauteur', 'xdata', 'ydata', etc.

Ce travail que j'ai fait n'a pas la vocation d'être pérenne, car il sera profondément affecté par la structure des données que Shahin va développer pour NW5. En revanche, il faut garder en tête que NW5 devra pouvoir manipuler

- dès maintenant des fichiers issus de NW4 ;

- à terme des fichiers de type EPANET (format .inp) ;

- voire des fichiers de type .json, car le format est très souple, mais peut générer de trop gros fichiers pour les grands réseaux.

J'ai par ailleurs défini un fichier .TEXTE qui est exotique. J'en avais besoin car c'est la façon dont j'ai communiqué avec Frédéric pour le travail sur le modèle *Cruscades*. L'idéal serait d'avoir, à part et si le besoin se fait sentir, un traducteur des divers formats .tpo, .dsg, .json, .Texte, etc. dans le format choisi pour les structures de données internes.

Un mote encore sur les fichiers .db. C'est le troisème type de fichier que NW4 a introduit. Il consigne les informations sur les équipements de type 'tuyau'. Il comporte beaucoup de champs qui en majorité ne sonct pas utilisés. Ces champs ont été créés à la demande de Gilles Corcos qui est l'hydraulicien à la base de la création de NW4. Quel champs retenir et/ou créer pour NW5 ? ***C'est une question à débattre avec nos collègues Denis et Olivier de l'INRAE***. 

# Interactions avec l'utilisateur et tests de validité

Ces deux points sont discutés dans les notes suivantes

- **Clients.md** Ce fichier passe en revue les divers mode d'utilisation des moteurs d'optimisation suivant les paramètres choisis par l'utilisateur. Ce fichier passe en revue la liste des paramètres (ce qui est dans la structure TP).

- **ArchitectureNW5.md** discute de l'architecture du projet NW5 (certains éléments sont repris et précisés au début de cette note) et aussi les **tests de validité dont l'importance est primordiale.**

Les tests de validité doivent être automatisés et effectués à partir de fichiers bien définis et invariables en localisation et en contenu. Ils doivent assigner des valeurs précises aux paramètres qui sont d'ordinaire choisis par l'utilisateur. Cela impose de bien séparer dans les modules de conception et de simulation ce qui peut être affecté par l'utilisateur (choix du fichier d'entrée –nom et adresse-, du format de sortie) et choix du moode d'action par modification des paramètres de design ou de simulation. Il y a deux phase. 

- **Phase 1 : Lecture de données**. Il faut deux types d'entrées. Soit l'utilisateur qui commence par donner les coordonnées, soit l'entrée des tests. Chacune de ces entrées vont activer le même fichier de lecture du fichier .tpo ou .dsg, ou autre. Si on est en mode 'utilisateur' on passe en phase 2 avec les données récupérées. Si on est en mode test on peut effectuer une validation automatique pour se garantir que d'éventuelles modifications dans le script de lecture de données produit toujours les résultats attendus.

- **Phase 2 : Exécution du programme**. Si on est en mode utilisateur, on commence par une interrogation de l'utilisateur sur le choix des paramètres puis on passe à l'exécution. Si on est en mode test on passe directement à l'exécution.

- **Phase 3 : Résultats et validation**.

