---
title: "NeatWork 5 – Modèles de comportement des clients"
author: "JPV, 8 mai 2026"
date: \today

pdf-engine: xelatex
mainfont: "Helvetica"
fontsize: 12pt
geometry:
  - margin=2cm
numbersections: true
#toc: true
#toc-depth: 2

header-includes:
  - \usepackage{setspace}
  - \setstretch{1.1}
  - \usepackage{fancyhdr}
  - \pagestyle{fancy}
  - \fancyfoot[C]{\thepage}
  - \fancyhead{}
  #pandoc types\ de\ clients.md -o types\ de\ clients.pdf --pdf-engine=xelatex

---

# Cas de figure pour NW5

## Clients : caractéristiques, comportement et impact sur les modèles de design et de simulation.

### Caractéristiques
Les clients sont sont rattachés à une borne. Il peut y avoir plusieurs clients à une même borne.
Ils sont caractérisés par deux paramètres et un mode d'association au réseau.

1. **Probabilité d'ouverture** quantifiée par `param.opentaps`
2. **Débit souhaité** quantifié par `vparam.targetflow`

Le traitement des débits servi à la borne dépend de deux spécifications

1. **Uniformité ou diversité** La différentiation se fait par examen des deux critères précédents. Si pour un seul client dans une quelconque borne a des caractéristiques qui le différencie d'au moins un autre client on impose `param.uniform_dem = 0`. L'autre valeur du paramètre est 1. (Ou bien True,False.)
1. **Débits bornés** Dans NW4 les débits n'ont pas de limite supérieure. Le débit chez un client dépend de deux valeurs : la pression en amont et les pertes de charges induites par son équipement (robinet  et orifice). Dans ce cas `param.bonded_flow = 0`. L'autre possibilité est que le débit reçu ne peut pas excéder la demande (param.targetflow) lorsque la pression excède la pression contractuelle. (Voir discussion après pour savoir où trouver cette pression.)

### Fonctionnement à la borne
Les deux modes de fonctionnement à une borne sont 

- le débit dépend de la pression, quelle qu'elle soit et de l'équipement du client. (mode passif de NW4)
- le débit est fixe et égal à valeur cible si la pression est supérieure à une pression de consigne ; sinon le débit est modulé comme dans le cas précédent. (mode irrigation NW5)

Dans le premier cas, ce qui régule les débits, ce sont les équipements des clients. Les pertes de charge induites pas ces équipements sont données par des paramètres techniques `param.faucetcoef`` pour les robinets et `param.orifcoef` et le diamètre intérieur de l'orifice spécifié (colone 5 ou 6 de la section !Nodes du fichier .dsg ).

Dans le deuxième cas, celui de débits bornés, on fait abstraction des équipements des clients. On suppose seulement qu'ils induisent une perte de charge égale à la pression contractuelle lorsque le débit est à sa valeur contractuelle. Si la pression est inférieure on fait une hypothèse sur la fonction de perte de charge des équipements des clients. (En cela on se rattache à la littérature qui parle de demande pilotée par la pression et de la fonction inverse que nous utilisons de charge induite par le débit.)

En théorie, les deux cas juste décrits peuvent s'appliquer aussi bien avec des clients uniformes ou différentiés. En fait, on ne considère que 3, voire vraiment 2 des 4 cas possibles.

1. **Cas 1** `param.bounded_flow = 0` et `param.uniform_dem = 1`. C'est le cas NW4. Tous les clients à une borne sont traités de la même façon. En effet la pression à la sortie de la borne. La connaissance du débit à la borne donne automatiquement le débit aux clients connectés. On peut ramener l'analyse au seul débit à la borne.
2. **Cas 2** `param.bounded_flow = 1` et `param.uniform_dem = 0`. C'est le cas du modèle irrigation. On fait abstraction des équipements physiques des utilisateurs. On fait seulement *l'hypothèse que les clients à une même borne réagissent de la même façon lorsque la pression est insuffisante.*
3.  **Cas 3** `param.bounded_flow = 1` et `param.uniform_dem = 1`. On le traite comme le cas pécédent. Si on est parti d'un modèle NW4, on oublie les caractéristiques des robinets et des orifices. (Dans un premier temps, j'avais ajouté la prise en compte des équipements connus, mais ce développement ne se justifie que s'il y a une demande des potentiels utilisateurs de NW5. Il n'y a pas de difficultés conceptuelles à traiter ce cas.)
4.  **Cas 4** `param.bounded_flow = 0` et `param.uniform_dem = 0`. Dans ce cas les clients réagissent différement à la pression à la borne si leurs orifices ne sont pas les mêmes. On peut tout de même prédire quelle sera le débit de chacun des clients relié à une même borne connaissant la pression, et donct on connaît le débit à la borne. On puet inverser cette fonction et en déduire la perte de charge (pression) à la borne comme fonction du débit à la borne. Ce cas n'est pas traitè.

En résumé il n'y a que deux cas. La chose à retenir est que la multiplicité des clients à une même borne est en partie la source des complexités. Une façon assez bourrin de contourner la difficulté est de se ramener au cas de réseaux avec des bornes avec un seul client. On crée pour cela des segments fictifs de longueur insignifiante équipées de tuyaux pas chers qui causeront une perte de charge négligeable parce que la lonueur est si faible. C'est ce que Laurent Drouet avait fait dans NW4.

Notons enfin, que lorsqu'il y a similitude de comportement à la borne, la satisfaction (débit servi/débit souhaité) est la même pour tous les clients à une même borne. On se contente donc de calculer une satisfaction à la borne.



## Liste des paramètres

`!Default properties`

`bescal,0.00089396`  
*paramètre technique de la fonction de friction. appartient à la structure partec* 

`bounded_flow,1`  
*paramètre de design et de simulation. Dans pardsg. (On peut simuler avec des paramètres différents de ceux du design)*

`faucetcoef,1.87E-08` 
*paramètre technique de friction des robinets. Dans pardsg.*
La formulation suivante est plus précise : =9.42/10000*1.27^(-0.219)


`fusion,0`  
*Pamètres de modification du réseau pour le module de design exclusivement. Il signifie que si un noeud est connecté à seulement deux autres nœuds, on remplace les deux arcs qui le relient à ces deuxnœuds par un seul arc de longueur égale à la somme des deux arcs qu'on supprime. Le nœud intemédiaire est également supprimé.
 ! la fusion doit respecter des contraintes de pression.
! Il faut gardere trace des fusions réalisées pour restituer le réseau initial si besoin.*

`lambda,1.781`    
*Paramètre technique, exposant du débit dans la fonction de perte de charge.*

`limitbudget,1000000000`.   
*Vieillerie de NW4. N'est plus pertinente. Seulement là pour préserveer la capacité de lecture par NW4.*

`nScenLoad,200000`.   
*Paramètre de design pour le calcul par échantillonage desfacteurs de charge*

`nu,4.781`.   
*Paramètre technique. Exposant du diamètre intérieur des tuyaux dans la fonction de perte de charge.*

`opentaps,0.5`.   
*Paramètre de design. Probabilité que le client se connecte au réesau. Lors d'une connexion son débit cible est défini par targetflow.*

`orifcoef,0.59`.   
*Paramètre technique pour le calcul des perte de charge dans les orifices. Valeur expérimentale établie par Gilles Corcos, UC Berkeley et co-fondateur d'APLV.*

`pipelength,6`.   
*Paramètre de design. Lorsqu'on fractionne un segment en deux sous-segments équipés detuyaux différents on souhaite utiliser des tuyaux dans leur longueur commerciale sans découpe. Si une découpe s'avère nécessaire, on essaie de la faire sur un seul des deux segments.*

`pressure,3`.   
*Paramètres de design utilisable dans le cas de débits bornés. Fait double emploi avec l'information dans la section !Bornes. On peut imaginer des pressions garanties différente par borne.*

`secu,1`.   
*Paramètre de design pour l'heuristique NW4 améliorée. Ce vacteur diminue fictivement l'altitude de la borne et donc de la force de gravité théoriquement disponible. Cela pousse l'heuristique à équiper le réseau avec des tuyaux plus conséquents, plus chers, mais rendant le système moins sensible aux pertes de charge. Les orifices doivent cependant être calculés par rapport aux vraies altitudes.*

`servicequal,0.65`  
*paramètre de design pour l'estimation des facteurs de charge (ou des débits estimés). il donne comme consigne que le débit estimé soit dans 65 % des cas supérieur au débit cdemandé par les les connexions tochastiques en aval.*

`seuil,0.06`  
*Vieillerie de NW4*

`split,0`  
*Paramètre de design. Lorsque le design à coût minimal propose de fractionner un segment en deux on peut considérer qu'il y a un nœud au raccord puisqu'il faut relier deux tuyaux de diamètres différents par un équipement. Denis Gilbert suggérait de faire apparaître de nœud et de créer deux arcs au lieu d'un seul fractionné. Cosmétique ?*

`targetflow,2`  
*Paramètre de design.*s

`uniform_dem,0`  
*Paramètre de design qui n'a peut-être pas lieu d'être parce qu'il découle immédiatement de la section !Clients des fichiers .tpo et .dsg*

`use_efficient_pipes,0`  
*Paramètre de design visant à ne considérer pour un arc donné que les tuyaux qui qui sont efficients du point de vue du coût. Le gain de temps de calcul est négigeable et ne justifie pas la complexité de modélisation.*

`vmax,9`  
*Paramètre de deign important. En principe aux alentour de 1.5 m/s. La viteese dans les tuyaux ne doit pas excéder cette valeur. Dans la phase de design on se base sur le débit estimé pour déterminer la vitesse résultant d'un choix d'un tuyau dans la base de données. Si on prend comme débit deréférence les débits cumulés de tous les clients en aval. la contrainte est très sévère (coût très grand). Si on prend le débit estimé par le facteur de charge, la protection est très insuffisante,*

`vmax_secu,0.95`  
*Paramètre de design complétant le précédent. Il agit comme servicequal, mais à 95% voire 99%. Il y a cependant une grosse difféence de sévérité de contrainte entre 95% et 100%. pour calculer le facteur de charge à 95% on pourrait se servir du bloc déjà écrit pour le cas standard à 65%. A partir de 95% on entre dans l'échantillonage d'événements rares. Il faut utiliser une variante d'échantillonage. Le code Matlab est écrit mais pas introduit dans GitHub. J'ai un peu oublié ce que j'y ai mis.*

`watertemp,20`  
*Vieillerie, mais ça doit jouer dans la fonction de friction. A voir avec les hydrauliciens de l'équipe.*




