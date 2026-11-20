# Tests MATLAB

Depuis la racine du depot, executer `setup`, puis appeler le test voulu par
son nom, sans extension. Un test termine sans erreur si ses assertions passent.
`test_processSol_smoke` est un script et efface les variables du workspace.

| Test | Fonctions concernees et controles principaux |
| --- | --- |
| `test_equivResist` | Resistance equivalente : arcs nus, existants en serie, renforcement en parallele, fictif, types des ID et entrees invalides. |
| `test_build_reinf_model` | Formulation : couts, filtrage des candidats au debit total, renforcement obligatoire pour vitesse excessive de l'existant seul, conflit avec `N="B"`, bornes de pression, contraintes hydrauliques, alignement des colonnes et optimum MILP connu. |
| `test_solveModel` | Resolution LP/MILP : melanges, activation des arcs, nettoyage numerique, couts, marges et rejet des modeles infaisables. |
| `test_reducePipeMixture` | Reduction a deux conduites : conservation de la resistance et cout minimal compare a une enumeration exhaustive. |
| `test_build_incidence_loads_cvar` | Charges : CVaR empirique et binomiale, ex aequo, cas limites, maintien du comportement VaR par defaut, charges exactes tous clients ouverts en VaR/CVaR et rejet de `opentaps=0`. |
| `test_client_data_precedence` | Lecture de fichiers TPO/DSG : priorite des clients sur les effectifs et parametres uniformes, heterogeneite preservee, orifices DSG inchanges, TPO sans clients, rejet du DSG sans clients et demandes simulees apres revue. |
| `test_physical_elevations` | Altitudes physiques conservees : reference de charge des arcs, premiere conduite montante avec pompe, translation des altitudes et exports JSON/DSG sans modification des altitudes ni des coordonnees. |
| `test_existing_pressure` | Pression hydrostatique de chaque troncon existant, egalite admissible, arcs nus et troncons nuls ignores, diagnostics detailles, independance de `N`, ordre et types des identifiants, classes invalides et pompe source. |
| `test_processSol_smoke` | Post-traitement : conduites physiques, longueurs, couts, equipement existant dans le second troncon et creation sans existant. |
| `test_constraint_satisfaction` | Diagnostics de pression : marges, deficits, correspondance des noeuds et exports JSON/DSG. |
| `test_reinforcement_integration` | Preparation du catalogue, enrichissement, resistances, graphe, charges, modele, resolution et post-traitement, avec lignes d'entree permutees. |

## Prerequis

- MATLAB avec les dossiers ajoutes par `setup`.
- Optimization Toolbox pour `test_solveModel` et `test_reinforcement_integration`.
  `test_build_reinf_model` ignore uniquement sa resolution MILP si cette toolbox
  est indisponible ; ses autres controles restent executes.
- Statistics and Machine Learning Toolbox pour `test_build_incidence_loads_cvar`.
  Le test d'integration utilise la branche CVaR a probabilite d'ouverture 1,
  sans appel aux fonctions binomiales ni tirage aleatoire.

## Test d'integration

```matlab
setup
test_reinforcement_integration
```

Reseau fictif : source S, jonction J, bornes B1 et B2. La liaison S-J est nue
et exige le grand diametre pour respecter la vitesse. J-B1 conserve son
existant ; J-B2 doit etre renforce pour satisfaire la pression. L'optimum
attendu coute 90, avec des marges finales de 47.5, 176.49 et 6.49 metres.
Les coefficients hydrauliques sont simplifies pour permettre un calcul manuel.

Ce test exerce la chaine de calcul sans lecture de fichier, interface,
export, modification de topologie ni simulation hydraulique finale.
Il utilise des renforcements d'un seul diametre : ce scenario ne valide pas
les ajustements de melanges ni les effets de l'arrondi commercial.
