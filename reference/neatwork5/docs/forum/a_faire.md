# À faire

Pense-bête évolutif — 19 novembre 2026. Projets en attente.

## 1. Conséquences des évolutions de la conception pour la simulation

- Conserver `TN.haut` physique ; traiter séparément charges hydrauliques et gains de pompe, dont `pump_pressure` à la source.
- Vérifier le transfert des clients, orifices et conduites parallèles du renforcement ; reconstruire le graphe du réseau exporté.
- Distinguer marges de conception avec `secu` et pressions simulées.
- Reprendre ces conventions dans le portage Python, avec des exemples de validation communs.

## 2. Clients homogènes par borne

- Contrôler l'homogénéité par groupe `TC.borne`, sans imposer l'identité entre bornes. Un client par borne est un cas particulier.
- Étendre `bounded_flow=0` à ce cas ; revoir les refus et conversions automatiques vers `bounded_flow=1`.
- Agréger débit et équipement : pour `n` robinets identiques ouverts et une cible individuelle `c` en m³/s, `Q_b=n*c` et `faucetcoef_b=n^2*faucetcoef`.
- Adapter ensemble `bineq`, marges/orifices de `processSol`, simulation et satisfaction. Aligner par identifiants ; les cibles ne deviennent pas des plafonds.
- Évolution non implémentée, à coordonner avec la personne chargée de la simulation Python. Les refus et conversions actuels restent en place en attendant.

## 3. Vanne régulatrice de pression

- Cadre initial : vanne sur un arc pont, sources en amont, aucune source physique aval ; conduite porteuse conservée en amont.
- Conception : agréger les demandes globalement ; séparer chemins de pertes, références de charge et pressions maximales amont/aval.
- Simulation à scénario fixé : aval à la consigne, puis amont avec sortie idéale plafonnée au débit aval. Si le plafond est atteint, assembler ; sinon recalculer le réseau global, vanne ouverte.
- Prévoir débit nul, tolérances, consigne et perte propre éventuelle.

Repère : [principes de gestion d'une vanne](ManageValve.md).

## 4. Pompe à l'intérieur du réseau

- Commencer par un gain de charge imposé ; courbes de pompe et limites de fonctionnement à étudier.
- **Conception ramifiée :** gain appliqué aux descendants concernés ; contrôler pression à l'entrée et pressions maximales avant/après pompe.
- **Simulation ramifiée ou maillée :** arc identifié, sens et relation de charge. En maillage, aucun découpage unique en descendants aval.
- Pompes et vannes : données d'équipement explicites, conservées à l'export/relecture, avec altitudes physiques intactes.

## 5. Distributions générales de demande

- Étendre conception et simulation à des lois de demande quelconques, au-delà du modèle « actif/inactif, demande fixe lorsque le client est actif ».
- Décrire la demande par client ou par borne agrégée, en conservant sa variabilité et les dépendances pertinentes entre demandes. L'approche NW5 de la simulation par les débits s'adapte bien à une fonction de demande pilotée par la pression. Le plus simple pour une borne est d'utiliser une fonction $\alpha Q^2$. Pour le module de conception, la pression de référence à la borne sera $\alpha \bar Q^2$, avec $\bar Q$ débit estimé à la borne pour une qualité de service donné. La fonction quadratique peut être remplacée par n'importe quelle fonction strictement monotone croissante et si possible différentiable pour des questions numériques dans l'algorithme IPM.
- Faire le pont vers une représentation de type EPANET, où les bornes regroupent de nombreux clients, tout en conservant une demande agrégée aléatoire.
- Adapter la génération des scénarios, les charges de dimensionnement et les indicateurs de satisfaction à ces distributions.
- Calculer `TL` et `TLv` ensemble dans `build_incidence_loads` : en Monte-Carlo, partager la base de scénarios et exploiter les deux niveaux `servicequal` et `vmax_secu`. Conserver la garantie `TLv >= TL`.
- Question ouverte : faut-il une méthode particulière pour estimer `TLv` lorsque le contrôle des vitesses vise des événements rares ? À discuter avec la personne chargée de la simulation Python. 
