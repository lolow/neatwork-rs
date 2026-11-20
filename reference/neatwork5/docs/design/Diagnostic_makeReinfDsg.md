**Diagnostic préalable de la chaîne makeReinfDsg**

17 novembre 2026 — référence examinée : `90ba6d8`, avec les fichiers locaux présents au moment de la revue.

**Conclusion**

La chaîne ne doit pas encore être figée comme référence pour le portage Python. Les corrections nécessaires concernent surtout les charges, les contrôles du réseau physique final et les échanges entre les fonctions. La lecture de la formulation LP/MILP, des résistances équivalentes et du solveur ne justifie pas une réécriture générale.

Les constats initiaux ci-dessous proviennent de la lecture du chemin actif et des tests, complétée par des calculs indépendants pour les contre-exemples numériques. Après connexion au VPN, les huit tests existants ont été exécutés sous MATLAB R2025a avec les licences Optimization Toolbox et Statistics and Machine Learning Toolbox disponibles : **8 tests sur 8 réussissent avant correction**. Leur premier lancement a nécessité un ajout manuel des chemins dans un lanceur temporaire, car `setup.m` échouait à la ligne 10 avec `MATLAB:catenate:dimensionMismatch`. Les corrections effectuées ensuite sont précisées dans les mises à jour et sections concernées.

**Mise à jour de `setup` : corrigé par l'utilisateur et vérifié sous MATLAB.** Aucun dossier nécessaire ne manquait. Les doublons `utils` et `tests/matlab` étaient inutiles mais non bloquants. La ligne entièrement commentée concernant `CreateDsg`, au milieu du tableau de cellules, interrompait la continuation et produisait deux rangées de tailles différentes. Sa suppression élimine l'erreur. Le `setup` actuel s'exécute et les chemins de `makeReinfDsg`, `readDesign` et `test_reinforcement_integration` sont correctement résolus. Une reproduction minimale a également confirmé la cause de l'ancienne erreur.

Trois vérifications supplémentaires ont reproduit les défauts du diagnostic avant correction : VaR à deux clients toujours ouverts, charge observée de 1 L/s au lieu de 2 sur chaque liaison ; pompe source avec première conduite montante, `pmax=10 m` au lieu de 20 ; longueurs 0.1 et 0.2 sur une liaison de 0.3, rejet `NW5:EquipmentLength`. Ces cas n'étaient pas couverts par les huit tests existants. Le dépassement de vitesse décrit plus bas reste, à ce stade, établi par calcul indépendant et lecture du code, sans reproduction MATLAB dédiée.

**1. Corrections touchant les calculs et leur interprétation**

**1.1. VaR uniforme : charge incorrecte lorsque tous les clients sont ouverts — corrigée et vérifiée.**

Dans la version initiale de [build_incidence_loads.m](../../matlab/CreateDsgDev/build_incidence_loads.m), lignes 232–248, la branche VaR initialisait `ld=1` et ne réalisait l'interpolation que si `p1>0`.

Pour deux clients dépendants, `opentaps=1` et `servicequal=0.95`, la loi du nombre de clients supplémentaires est déterministe : `k=1`, `p1=0`, `p2=1`. Aucune branche de mise à jour n'est activée ; la charge reste celle d'un seul client au lieu de deux. Le défaut concerne aussi les troncs avec davantage de clients. `opentaps=1` et `use_cvar=0` sont les valeurs par défaut de `extract_parameters`.

Correction effectuée selon la règle métier précisée par l'utilisateur : `opentaps=0` provoque un arrêt explicite `NW5:LoadProbability`, car aucun client n'est jamais ouvert et il n'y a pas de demande à dimensionner. La probabilité commune est validée dans `(0,1]` avant les branches VaR/CVaR. Avec `opentaps=1`, la charge vaut directement `1+nbdep1`, soit tous les clients dépendants du sous-ensemble concerné. Pour `0<opentaps<1`, les calculs probabilistes sont conservés.

Validation MATLAB réussie : `test_build_incidence_loads_cvar`, complété pour vérifier le rejet de zéro en VaR et CVaR, même avec un seul client, et les charges exactes tous clients ouverts sur un arbre comportant des liaisons à un et plusieurs clients. Les niveaux de service 0.5, 0.95 et 1 sont couverts pour ce dernier cas. `test_reinforcement_integration` réussit également. Les anciens tests CVaR qui acceptaient zéro ont été remplacés par le contrôle de rejet.

**1.2. Priorité des clients sur les effectifs et les paramètres d'entrée — corrigée et vérifiée.**

Dans le code initial, lorsqu'une section clients était présente, [parseTpoToTables.m](../../matlab/InOut/parseTpoToTables.m) conservait le `nbrob` de `!Nodes`, sans le réconcilier avec `TC`. Les charges uniformes étaient ensuite calculées avec `dat.M_path * TA.nbrob` dans `build_incidence_loads`. Les paramètres globaux lus pouvaient aussi contredire les probabilités et cibles de `TC`.

Deux clients identiques affectés à une borne déclarée avec `nbrob=1` peuvent donc être dimensionnés comme un seul client, tandis que les deux clients sont conservés pour la simulation.

**Règle retenue par l'utilisateur : avant la revue des paramètres, `TC` fait autorité.** Les effectifs `TN.nbrob` sont reconstruits depuis `TC`, indépendamment de l'ordre des lignes et du type des identifiants. L'indicateur `uniform_dem` est déduit des probabilités et cibles réellement lues, même si le fichier contenait un indicateur différent. Pour une demande uniforme, `opentaps` et `targetflow` sont remplacés par les valeurs communes de `TC.proba` et `TC.cible`. Les paramètres de demande restent numériques, notamment `uniform_dem=0/1`, pour préserver le contrat d'export.

Cette règle est partagée par les parseurs TPO/DSG et le lecteur `readDesign` via [apply_client_data.m](../../matlab/InOut/apply_client_data.m). La table `TP` reste la transcription brute des paramètres du fichier ; les calculs utilisent les structures `param`/`pardsg` mises en cohérence avec les clients.

**Différence entre TPO et DSG :** un TPO sans section clients conserve la construction de `TC` depuis les effectifs de `TN` et les paramètres complétés. Un DSG sans `!Clients` conserve son erreur explicite. Les colonnes d'orifices du DSG ne sont jamais interprétées comme des effectifs. La reconstruction des anciens DSG NW4 par [reserve/maj_NW4_NW5.m](../../reserve/maj_NW4_NW5.m) reste un travail distinct, dont le diagnostic est reporté ; cet utilitaire n'est pas intégré au parseur.

**Après la revue, les paramètres retenus font autorité sur la demande uniforme :** dans `makeReinfDsg`, la mise à jour de `TC.proba` et `TC.cible` est placée immédiatement après `review_design_parameters_reinf`. Les demandes hétérogènes restent individuelles. La simulation `genscen` actualise également les cibles uniformes depuis `targetflow`, en plus de l'actualisation existante des probabilités, pour respecter les choix de la revue de simulation.

Validation MATLAB : `test_client_data_precedence` lit de vrais fichiers temporaires TPO/DSG et vérifie les effectifs, les paramètres uniformes, les demandes hétérogènes, les orifices DSG conservés, la génération TPO sans clients, le rejet du DSG sans clients et les demandes simulées après modification des paramètres. Ce test, `test_build_incidence_loads_cvar` et `test_reinforcement_integration` réussissent.

**1.3. Vitesses du renfort : filtrage conservateur au débit total.**

Dans la version initiale, [build_reinf_model.m](../../matlab/CreateDsgDev/build_reinf_model.m) sélectionnait chaque candidat avec le débit qu'il porterait s'il équipait seul le renforcement parallèle à l'existant. Après mélange de deux diamètres, [processSol.m](../../matlab/CreateDsgDev/processSol.m) supprimait le raccordement intermédiaire à l'existant en ajustant les longueurs. La résistance équivalente était conservée, mais le débit du petit tronçon pouvait augmenter par rapport à celui utilisé pour son admissibilité.

Contre-exemple à débit total de contrôle fixé, calculé indépendamment en Python :

- Liaison de 100 m ; exposant hydraulique 2 ; résistance linéaire existante 1.
- Candidats de diamètres 0.05 et 0.10 m, résistances 16 et 1, coûts 1 et 2 par mètre.
- Débit total 9.817477 L/s ; limite de vitesse 1.01 m/s.
- Résistances équivalentes des candidats homogènes : 0.64 et 0.25. Un budget correspondant à 0.445 impose au maximum une moitié de petit diamètre. Les coûts rendent ce mélange moitié/moitié optimal dans le modèle, pour un coût de 150.

| Étape | Longueurs petit / grand | Vitesse dans le petit diamètre | Coût ajouté |
| --- | ---: | ---: | ---: |
| Filtre des candidats homogènes | Chaque candidat examiné séparément | 1.000000 m/s ; grand : 0.625000 m/s | — |
| Suppression du raccordement | 20.100188 / 79.899812 m | 1.664584 m/s | 179.899812 |
| Arrondi commercial à 6 m | 16 / 84 m | 1.758144 m/s | 184 |

Dans cet ancien cas de filtrage avec partage, les pertes de charge diminuaient après l'arrondi, tandis que la vitesse dans le petit diamètre augmentait. Le contrôle des seules marges de charge ne détectait donc pas le dépassement. Le commentaire de `processSol`, qui décrivait un filtrage conservateur avec le débit total, ne correspondait alors pas au constructeur du modèle.

**Choix de l'utilisateur, appliqué le 18 novembre 2026 :** les tuyaux candidats sont désormais filtrés comme s'ils devaient transporter seuls tout le débit `Qvmax` calculé depuis `TLv`, soit `share=1` pour le test de vitesse. Chaque diamètre admis peut ainsi transporter le débit total ; les mélanges et arrondis ne réintroduisent pas le dépassement discuté dans le renfort au débit de contrôle retenu. Le calcul des résistances conserve la contribution physique de l'existant dans `TR` : ce choix ne modifie que le filtre d'admissibilité. Il peut être plus coûteux que le filtrage avec partage.

**1.4. Pompe à la source et conservation des altitudes physiques — corrigées et vérifiées le 18 novembre 2026.**

Dans le code initial, [apply_pump_pressure.m](../../matlab/CreateDsgDev/apply_pump_pressure.m) conservait la source à zéro et retranchait le gain de charge aux autres nœuds. [enrichTablesReinf.m](../../matlab/CreateDsgDev/enrichTablesReinf.m) calculait ensuite `pmax` à partir de ces hauteurs transformées.

Exemple : source d'altitude relative 0 m, premier nœud à +10 m, gain de pompe 20 m. Les hauteurs internes deviennent 0 et −10 ; le code obtient `pmax=10`. L'enveloppe hydrostatique aux deux extrémités est pourtant de 20 et 10 m : la conduite exige une classe d'au moins 20 m selon ce critère. Aucun contrôle topologique actif n'exclut ce réseau.

Correction autorisée par l'utilisateur : `TN.haut` conserve désormais les altitudes physiques dans toute la chaîne de conception. `makeReinfDsg` mémorise `dat.source_height`, sans décaler `TN` et sans appeler `apply_pump_pressure`. Il transmet directement `dat.source_height` au sixième argument `source_height` de `enrichTablesReinf`.

La fonction calcule `Hsource = source_height + pardsg.pump_pressure`, puis `TA.pmax = Hsource - min(haut_pred, haut_succ)` à partir des altitudes physiques. Après la jointure du nœud aval, seule la donnée de calcul `TA.haut` devient `z_aval-Hsource`.

L'export `data4DsgFile` transmet les altitudes et coordonnées physiques telles quelles. Les anciennes opérations de restitution et de normalisation de la source à l'origine ont été retirées ; `TN = TNodes` est conservé. Le fichier `apply_pump_pressure.m` reste présent mais n'est plus appelé par le pilote de conception.

Validation MATLAB réussie : `test_physical_elevations` contrôle les sources physiques à 100 et 350 m, les gains de pompe 0 et 20 m, des tables permutées et les exports réels JSON/DSG avec ou sans métadonnée `dat.source_height`. Dans le cas source 100 m, premier nœud 110 m et gain 20 m, la première conduite reçoit bien `pmax=20 m`. Les altitudes et coordonnées sont préservées à l'export. `test_reinforcement_integration` retrouve les mêmes choix, coût et marges avec source à 0 ou 100 m et conserve la géométrie jusqu'à `processSol`. `test_constraint_satisfaction` réussit également, avec l'avertissement de déficit prévu par son scénario.

**1.5. Contrôle de la pression hydrostatique de l'existant — ajouté et vérifié le 18 novembre 2026.**

Dans le code initial, le filtre de [build_reinf_model.m](../../matlab/CreateDsgDev/build_reinf_model.m) ne portait que sur les nouveaux candidats. Un existant dont la classe était insuffisante pouvait être conservé. Un renforcement en parallèle ne supprime pas sa surpression hydrostatique.

La fonction dédiée [validate_existing_pressure.m](../../matlab/CreateDsgDev/validate_existing_pressure.m) est désormais appelée par `makeReinfDsg` immédiatement après `enrichTablesReinf`, avant les résistances équivalentes, le graphe et l'estimation des débits `TL`/`TLv`. Chaque tronçon existant de longueur positive doit satisfaire `TD.pressure(ID) >= TA.pmax`. Les tronçons absents et les arcs nus sont ignorés ; les identifiants sont rapprochés du catalogue indépendamment de son ordre.

Une insuffisance arrête le dimensionnement avec `NW5:ExistingPressure`. Le message liste toutes les sections concernées avec les extrémités de liaison, le numéro du tronçon, l'identifiant du tuyau, la pression requise et la pression admissible en mètres de charge. Ce contrôle ne dépend ni de `N` ni de `y` et ne peut pas être contourné en autorisant un renforcement. Une classe consultée invalide provoque une erreur de données ; une classe `+Inf` reste admise comme non contraignante.

Validation MATLAB réussie : `test_existing_pressure` couvre les classes suffisantes et l'égalité, les insuffisances des deux tronçons d'une même liaison, les arcs nus et longueurs nulles, les ordres et types d'identifiants, le statut `N="B"`, une classe invalide et une limite infinie. Le cas source à 100 m, premier nœud à 110 m, gain de pompe 20 m refuse une conduite de classe 15 m et accepte la classe 20 m. `test_reinforcement_integration`, qui appelle désormais ce contrôle au même endroit que le pilote, conserve ses choix, son coût et ses marges de référence.

**Vitesse de l'existant seul — règle appliquée le 18 novembre 2026 :** pour tester l'absence de renforcement, prendre `share=0` du côté renfort, donc tout `Qvmax` dans l'existant. Le constructeur compare le diamètre minimal requis à chacun des tronçons existants de longueur positive. Si un tronçon est insuffisant, il impose `y=1`, comme pour un arc nu. Les tronçons de longueur nulle sont ignorés ; les identifiants sont reliés au catalogue, sans supposer son ordre.

Si ce renforcement obligatoire est interdit par `N="B"`, arrêter avant résolution avec `NW5:ReinforcementBlocked` et les nœuds aval des liaisons concernées. Un arc bloqué dont l'existant suffit reste admis. Ce mécanisme impose l'ajout d'un renfort ; la vitesse résiduelle dans l'existant après partage est un contrôle distinct. La pression hydrostatique est désormais vérifiée en amont par la fonction dédiée.

Validation MATLAB réussie : `test_build_reinf_model` contrôle le filtrage au débit total, l'obligation déclenchée par `TLv` même si les contraintes de charge permettent de garder l'existant seul, le petit diamètre en second tronçon, les tronçons absents ignorés, l'équipement porté seulement par `lon2` et le diagnostic des conflits avec `N="B"`. L'optimum MILP de référence reste de 50. `test_reinforcement_integration` réussit également, avec son optimum de 90 et les marges attendues.

**1.6. Les paramètres doivent être validés avant leur emploi.**

La révision interactive ne valide systématiquement que `pump_pressure` et `pipelength`. En particulier, `secu>=1` n'est pas imposé. Le modèle et le post-traitement divisent le budget terminal par `max(secu,eps)` : `secu=0.5` le double, tandis qu'une valeur nulle ou négative le gonfle considérablement.

Les probabilités et cibles des clients ne sont pas davantage contrôlées dans la branche Monte-Carlo : une probabilité supérieure à 1 devient silencieusement « toujours ouvert », une probabilité négative « jamais ouvert », et une demande négative peut annuler une autre demande à la même borne.

Correction : centraliser la validation des données numériques après lecture et après révision interactive. Contrôler notamment `secu` fini et supérieur ou égal à 1, les probabilités individuelles dans [0,1], les demandes finies non négatives, les indicateurs binaires, `vmax` strictement positif et `nScenLoad` entier positif fini. La probabilité commune `opentaps` du dimensionnement uniforme est désormais validée dans `(0,1]` conformément au point 1.1.

**2. Corrections des échanges, de la lecture aux exports**

| Constat établi | Déclencheur et conséquence | Correction proposée |
| --- | --- | --- |
| Initialisation des chemins avec `setup` — corrigée et vérifiée | Ancien code : ligne entièrement commentée au milieu de `folders`, interrompant la continuation et produisant `MATLAB:catenate:dimensionMismatch`. | L'utilisateur a supprimé cette ligne et deux doublons. Vérification réussie depuis une nouvelle session MATLAB ; aucun dossier nécessaire ne manquait. |
| Dossier de sortie créé trop tard | `data4DsgFile`, ligne 55, écrit dans `out/<nom>` ; le `mkdir` du pilote intervient après. Premier export d'un nouveau nom impossible si le dossier est absent. | Créer le dossier avant toute écriture. Màj : export réussi dans un dossier initialement absent.|
| JSON exporté non relisible par le lecteur partagé | `save_dsg_json` enregistre `TB`, mais `read_Dsg_json`, lignes 37–38, ne la reconstruit pas. `readDesign`, ligne 80, demande ensuite `S.TB`. | Rétablir le contrat du lecteur JSON et tester le passage vers la simulation. *Correction faite.*|
| Catalogue DSG exporté dans un format incompatible | `prepareReinforcementTables` ajoute `resist` et `cond`. `data4DsgFile`, lignes 114–115, exporte tout `TD` et ajoute un marqueur. Le lecteur DSG attend les huit champs persistants puis le marqueur, soit neuf champs au total. | Sélectionner et ordonner explicitement les champs persistants ; normaliser `type`/`Type` selon l'origine du catalogue. *Changements faits*|
| Statut des conduites nouvelles incompatible avec une reprise | `processSol`, ligne 76, transforme `N` en `R`. L'export conserve `R`, mais `prepareReinforcementTables`, ligne 129, n'accepte que `N`/`B`. Une création neuve arborescente est ainsi rejetée à la reprise, même sans parallèles. | Séparer la nature historique de la conduite de son autorisation de renforcement, ou normaliser explicitement ce statut à l'import. Màj : Acceptation de 'R'|
| Sortie MAT cassée | `data4DsgFile`, ligne 140, supprime quatre variables absentes de la table physique produite par `processSol`, puis échoue avant `save`. La branche MAT de `readDesign`, ligne 61, omet aussi de restituer `TB`. | Aligner les contrats MAT si ce format reste annoncé ; sinon le retirer explicitement des sorties disponibles pour cette version. Màj : Retrait explicite avant le dialoque d'export.|
| Coordonnées de la source — normalisation supprimée au point 1.4 | L'ancien bloc de `data4DsgFile` forçait la source à (0,0), en conservant les autres coordonnées. | Altitudes et coordonnées sont désormais exportées sans transformation. *Fait dans le bloc 1.4*.|
| Annulation d'export non gérée | `data4DsgFile`, lignes 49–51, retourne sans affecter `filename`, alors que le pilote en demande la valeur. | Retour d'annulation explicite, puis arrêt cohérent de l'étape d'export et des métadonnées. *Si filename manque, l'exécution s'arrête sans export, mais l'espace de travail et préservé.* |
| Longueurs décimales valides rejetées | `prepareReinforcementTables`, ligne 133, et `equivResist`, ligne 37, exigent une égalité exacte. En arithmétique flottante, 0.1 + 0.2 diffère de 0.3. | Harmoniser une tolérance absolue/relative avec les contrôles de `processSol`. *Changements faits.*|
| Réseau minimal source → borne rejeté | `enrichTablesReinf`, lignes 62–65, écrase le statut terminal de l'arc adjacent à la source. `build_incidence_loads`, lignes 70–71, rejette alors son client. | Conserver le statut du nœud aval ; représenter séparément le rôle d'arc adjacent à la source. *enrichTables 66-69 commentées pour être supprimées.*|
| Cas de demande accepté en amont puis refusé tardivement | `review_design_parameters_reinf` laisse disponibles les demandes hétérogènes non bornées si chaque borne a un client. `build_reinf_model`, lignes 198–205, les rejette toutes. | Extension reportée : clients homogènes au sein de chaque borne, sans imposer l'identité entre bornes. Adapter ensemble contraintes, marges/orifices et simulation, avec la personne chargée de la simulation Python. Voir [le pense-bête, section 2](../forum/a_faire.md#2-clients-homogènes-par-borne). Des commentaires rappellent ce travail dans le code ; les refus et conversions actuels restent inchangés. |
| Valeurs par défaut de champs de pardsg | Dans review_parameter_design le champs correspondant à fusion, split,use_efficient_pipes et use_cvar commandent des options de design qui ne doivent pas être activées. D'autres champs hérités de NW4 ne correspondent plus à rien. | Mettre à zéro les valeurs par défaut. Ne pas présenter les deux champs inutiles dans le prompt. *Fait*. |

Le JSON et le MAT ne sont pas actuellement des entrées admises par `makeReinfDsg`. Leur relecture reste néanmoins pertinente pour les consommateurs des résultats, notamment la simulation MATLAB qui appelle `readDesign`. Corriger le lecteur partagé n'oblige pas à élargir les entrées de conception.

Le lecteur JSON ignore aussi plusieurs tables de diagnostics enregistrées par l'écrivain. Il convient de décider lesquelles doivent être restituées pour l'inspection des résultats ; cette question est distincte du blocage immédiat sur `TB`.

**3. Points de méthode à fixer avant le portage**

**Reproductibilité des demandes hétérogènes.** Les deux appels de calcul de charges pour `TL` et `TLv` produisent deux tirages indépendants. Ce choix n'est pas intrinsèquement incorrect ; le pilote impose ensuite `TLv>=TL`. En revanche, ni les scénarios ni l'état aléatoire ne sont sauvegardés. Pour les comparaisons MATLAB/Python, prévoir un jeu de scénarios fixe, indexé par identifiant client. Une même graine dans les deux langages ne garantit pas à elle seule un même jeu de tirages.
Piste proposée : produire `TL` et `TLv` dans un même appel à `build_incidence_loads`, avec une base de scénarios commune dans la branche Monte-Carlo et les deux niveaux `servicequal` et `vmax_secu`. Les agrégations et tris pourraient ainsi être partagés, en conservant `TLv>=TL`. La branche uniforme utilise une loi binomiale, sans scénarios à partager. Reste ouverte la question d'une méthode particulière pour `TLv` lorsque les événements visés sont rares ; partager les scénarios ne résout pas à lui seul leur estimation. Évolution non implémentée, à discuter avec la personne chargée de la simulation Python ; voir [le pense-bête, section 5](../forum/a_faire.md#5-distributions-générales-de-demande).

**Conditionnement des charges.** La branche uniforme suppose un robinet déterminé ouvert : `1+Bin(n−1,p)`. La branche hétérogène retient les scénarios dans lesquels la demande totale de la borne est positive. Pour plusieurs clients par borne, ces événements diffèrent. Il faut expliciter la convention métier ; je ne propose pas de changer automatiquement cette méthode dans une correction informatique.

**Demande toujours nulle.** La branche uniforme prévoit un équipement minimal ; une borne hétérogène sans scénario actif provoque une erreur explicite. Le comportement souhaité doit être documenté et testé.

**Altitudes physiques et données de calcul.** La séparation est mise en place au point 1.4 : `TN.haut` reste physique, tandis que `TA.haut` exprime l'écart à la charge de source. L'export ne restaure plus de hauteurs. Ce choix prépare la distinction entre géométrie, références hydrauliques, chemins de pertes et budgets nécessaire à la vanne.

**4. Ce que je conserverais**

À la lecture, les résistances équivalentes, l'agrégation des pertes sur les chemins, l'exclusion du tuyau fictif des investissements et la distinction LP/MILP sont cohérentes avec le modèle documenté. La réduction des mélanges et la suppression du raccordement intermédiaire visent explicitement la conservation de la résistance ; les coûts après transformation sont déjà distingués du coût optimisé.

Les marges négatives après ajustement sont conservées, signalées et exportées. Leur existence est compatible avec la politique actuelle de diagnostic ; une correction ne doit pas les masquer.

Les restrictions sur les arbres, les parallèles en entrée de conception, la source unique et son degré, ainsi que le report de la fusion et du découpage, sont explicites. Les élargir ne fait pas partie du préliminaire nécessaire à la vanne.

**5. Validation et ordre de travail proposés**

Les huit tests existants passent après contournement de l'erreur de `setup`. Le test `test_constraint_satisfaction` émet l'avertissement de déficit de pression prévu par son scénario, puis réussit. Le test d'intégration commence avec des tables en mémoire et s'arrête à `processSol`. Les tests d'export appellent directement les écrivains ; ils ne traversent ni `data4DsgFile` ni la relecture complète. Ils ne peuvent donc pas détecter plusieurs des ruptures de contrat recensées ici.

Je propose les étapes suivantes, chacune accompagnée de sa validation :

1. Corriger les charges VaR et la cohérence des comptes de clients ; centraliser les validations numériques.
2. Corriger le calcul des pressions maximales avec pompe et ajouter les diagnostics de pression de l'existant et de vitesse du réseau final.
3. Fiabiliser le passage conception → export → lecture simulation, en priorité JSON. Corriger le DSG si sa réutilisation reste attendue ; réparer ou retirer clairement le MAT.
4. Rendre la chaîne exécutable sans dialogue à partir d'une configuration, avec le script interactif comme pilote. Cela permet de tester le vrai enchaînement sans changer l'algorithme.
5. Figer les données et résultats de référence, puis seulement créer l'extension vanne sur cette base.

| Cas de validation | Résultat à contrôler |
| --- | --- |
| Deux clients et tous les robinets ouverts, VaR puis CVaR | Charge de deux clients, indépendamment du chemin utilisé. |
| Clients explicites et compte `nbrob` discordant | `TC` fait autorité ; `TN.nbrob` est reconstruit en conséquence. |
| Section clients absente dans un TPO | `TC` créée à partir de `TN` et des paramètres complétés. |
| Section clients absente dans un DSG, notamment NW4 | Erreur explicite maintenue ; conversion externe et diagnostic de l'utilitaire reportés. |
| Paramètres du fichier contredisant une `TC` uniforme | `uniform_dem`, `opentaps` et `targetflow` déduits de `TC` avant la revue. |
| Demande uniforme modifiée après la revue | Probabilités et cibles de `TC` actualisées ; demandes hétérogènes préservées. |
| Renforcement à deux diamètres, avant/après arrondi | Résistance, coût, marges et vitesses des conduites physiques finales. |
| Source non nulle, première conduite montante, gain de pompe | Altitudes et coordonnées préservées ; classe de pression du premier arc correctement calculée. |
| Existant de classe insuffisante | Diagnostic persistant, même si les contraintes de charge sont satisfaites. |
| Réseau neuf et dossier de sortie absent | Export créé puis relu par le consommateur. |
| Export DSG et reprise d'un arbre sans parallèles | Catalogue, identifiants et statuts compatibles. |
| Export JSON et lecture pour simulation | Tables nécessaires complètes, en particulier `TB`. |
| Réseau source → borne et longueurs décimales | Cas valides acceptés. |
| Annulation et paramètres invalides | Sortie propre ou rejet explicite avant calcul. |
| Demandes hétérogènes et scénarios conservés | Résultats reproductibles et utilisables pour comparer MATLAB et Python. |

La référence pourra être figée lorsque les corrections retenues auront été exécutées et validées sous MATLAB, et que les formats maintenus auront traversé un véritable aller-retour. Les résultats actuels établissent le comportement avant correction : réussite des cas déjà couverts et reproduction de trois défauts supplémentaires. Après correction, il faudra vérifier à la fois ces nouveaux cas et la non-régression des huit tests existants.
