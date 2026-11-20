---
title: "NeatWork 5 – Architecture de NW5 et tests de validité"
author: "JPV"
date: \today

pdf-engine: xelatex
mainfont: "Helvetica"
fontsize: 12pt
geometry:
  - margin=2cm
numbersections: true
toc: true
toc-depth: 2



header-includes:
  - \usepackage{setspace}
  - \setstretch{1.1}
  - \usepackage{fancyhdr}
  - \pagestyle{fancy}
  - \fancyfoot[C]{\thepage}
  - \fancyhead{}
---

# Architecture de NW5 et tests de validation

L'originalité de NW5 est dans le traitement d'une demande stochastique pour le dimensionnement et pour la simulation, ce que ne fait pas EPANET. En revanche EPANET permet de études sur des séries chronologiques. NW5 devra pouvoir le faire dans le futur.

## Architecture de NW5

On peut envisager l'architecture comme un système de poupées russes emboîtées en plusieurs couches

### Couche $C_1$

$C_1$ comprend deux codes d'optimisation. Il s'agit essentiellement ***algorithmique numérique***.
1. Un programme linéaire pour le module de conception/dimensionnement. Dans Matlab, je me sers de linprog( ). Il existe des équivalents en logiciel libre. Il y a glpk, mais je pense qu'il existe une version développée par le MIT plus efficace.
2. Un programme non-linéaire IPM que j'ai développé et qui semble faire preuve d'une bonne stabilité et efficacité (comparaisons avec Mosek).

### Couche $C_2$
Cette couche construit les modèles pour alimenter les moteurs. Le travail relève de la ***modélisation de Recherche Opérationnelle***. La mise en œuvre est un mélange de l'algorithmique numérique et de l'algorithmique transactionnelle (néologisme de jpv).

#### Module de design (conception/dimensionnement)
Il y a deux approches. Une seulement est programmée en Matlab.

1. Heuristique NW4 améliorée, basée sur les facteurs de charge. Le calcul des facteurs de charge se fait   
    1. soit par exploitation de lois de Bernouilli dans les bibliothèqus statistiques de Matlab et de Python.
    2. soit par estimation statistique Monte-Carlo.
2. Soit par optimisation robuste. Programmée en AMPL mais pas intégrée au code Matlab.

Les modèles doivent pouvoir traiter plusieurs cas d'utilisation (type NW4 ou irrigation, mais pas seulement). Le modèle permet d'appeler la couche $C_1$.  La design ne s'applique qu'à des réseaux arborescents.

#### Module de simulation
1. **Génération de scénarios**
2. **Calcul des débits par scénario**
3. **Traitement statistique des résultats**

La simulation s'applique à des ***réseaux arborescents, maillés et multi-sources***.

Le traitemnt de cette couche empiète sur les deux coudhes suivantes, en particulier parce que j'ai dû avoir recours à des interactions avec utilisateur pour préciser le typde de résultat souhaité (par ex. NW4 vs irrigation). Plus fondamentalement, j'ai dû créer des structures pour échanger les informations provenant de la couche supérieure $C_3$ pour créer les modèles et transférer ces modèles à la couche interne $C_1$.

### Couche $C_3$
Il s'agit d'aller chercher les données du projet à traiter et de transmettre les informations pour activer la couche $C_2$.

#### Approche Matlab
Je me suis inspiré de NW4 qui résume les informations dans des fichiers texte .tpo et .dsg. Pour la simulation, j'ai enrichi la lecture d'un réseau dimensionné en définissant des format .json, .txt, .mat, avec l'idée qu'on doit aussi lire des format type EPANET .inp. Gros travail de lecture (parser) qui risque de n'être pas pérène. 

#### Approche Python
A discuter entre Sa. et Sh.

### Couche $C_4$
Système de base de données. Interaction I/O et utilisateur/logiciel.

## Tests de validation automatiques
Il faut une petite restructuration du module de simlation pour bien séparer ce qui universel (simulEngine, loadData) et ce qui relève soit du choix de l'utilisateur (simulInteractive), soit des tests de validation (runTest).

Dans le cas de runTest, je préconise une sortie en .json des indicateurs clé des résultats à comparer avec ceux qui servent d'étalon dans un même format .json.







# Précision sur les tests

Oui, c’est exactement le bon principe, avec toutefois quelques précautions importantes.

Pour les quantités flottantes (`double`) comme :

* débits,
* pressions,
* valeur du critère,
* résidus,
* coûts,

il ne faut **jamais** tester :

```matlab
x == xref
```

mais utiliser une tolérance.

La pratique standard est :

```matlab
abs(x - xref) <= atol + rtol * abs(xref)
```

avec :

* `atol` : tolérance absolue,
* `rtol` : tolérance relative.

## Pourquoi combiner absolu et relatif ?

La seule erreur relative :

```matlab
abs(x - xref) / abs(xref)
```

pose un problème si `xref ~ 0`.

Exemple :

```matlab
xref = 1e-14
x    = 1e-12
```

L’erreur relative devient énorme alors que les deux valeurs sont numériquement “nulles”.

Donc on combine les deux.

## Recommandation pratique pour NW5

Je conseillerais :

```matlab
rtol = 1e-8;
atol = 1e-10;
```

ou même :

```matlab
rtol = 1e-7;
atol = 1e-9;
```

au début du projet Python.

Pourquoi pas plus strict ?

Parce que :

* MATLAB et Python ne feront pas forcément les opérations dans le même ordre,
* certains solveurs linéaires changent les arrondis,
* sparse Cholesky / LU peuvent produire de petites différences,
* BLAS/LAPACK diffèrent suivant les plateformes.

Donc `1e-12` est souvent trop ambitieux pour des tests inter-langages.

## Pour les entiers

Pour :

* nombre d’itérations,
* nombre de scénarios actifs,
* taille des ensembles,

vous pouvez tester exactement :

```matlab
iter == iter_ref
```

Mais attention :

le nombre d’itérations peut changer après :

* modification du préconditionnement,
* changement de solveur,
* amélioration du critère d’arrêt.

Donc il faut distinguer :

### Quantités physiques essentielles

à tester strictement :

* débits,
* pressions,
* conservation,
* objectif.

### Quantités algorithmiques

à tester plus souplement :

```matlab
iter <= iter_max
```

ou :

```matlab
abs(iter - iter_ref) <= 2
```

## Très bonne pratique

Pour les vecteurs :

```matlab
err = norm(x - xref, inf);
tol = atol + rtol * norm(xref, inf);
ok = err <= tol;
```

Norme infinie :

```matlab
max(abs(...))
```

souvent idéale pour les validations.

## Structure typique

```matlab
function ok = compareDouble(x, xref, rtol, atol)
err = norm(x - xref, inf);
tol = atol + rtol * norm(xref, inf);
ok = (err <= tol);
end
```

## Point très important pour NW5

Si vous utilisez Monte-Carlo :

* imposez systématiquement :

```matlab
rng(seed)
```

sinon les tests seront non reproductibles.

Et idéalement :

```matlab
opts.seed = 12345;
```

dans chaque cas de test.
