---
title: "NeatWork 5 – Format des documents de travail"
author: "JPV, 8 mai 2026"
date: \today

pdf-engine: xelatex
# mainfont: "Helvetica"
mainfont: "Arial"
fontsize: 12pt
geometry:
  - margin=2cm
numbersections: true
toc: true
toc-depth: 2
vheader-includes:
  - \usepackage{setspace}
  - \setstretch{1.1}
  - \usepackage{fancyhdr}
  - \pagestyle{fancy}
  - \fancyfoot[C]{\thepage}
  - \fancyhead{}
---

# Pourquoi choisir un format pour les documents de travail
Mes contributions au développement de NeatWork ont été essentiellement théoriques. Matlab m'a permis de développer des prototypes d'algorithme. Maintenant, on passe à du travail collaboratif professionnel. Parmi les nombreuses exigences que cela pose, il y a la nécessité de commenter et d'introduire de nombreux README.md et de partager des documents de travail.

***Tout ce qui suit enfonce des portes ouvertes pour la plupart d'entre vous !!!***

## Format MarkDown .md
J'ai découvert le format .md en copiant les réponses de l'IA. Elles étaient formatées sur l'écran 
mais une fois copiées dans un éditeur TextEdit ou Word le format (Sections, caractères gras ou italiques, liste …) disparaît. Puis j'a constaté que Matlab donnait le visuel formaté. Pour finir j'ai trouvé dans mon environnement trois éditeurs qui formatent et permettent une meilleure visualisation.

- Visual Studio Code (gratuit)
- Github
- Matlab

Le premier se révèle extraordinairement puissant et adapté. J'ai découvert qu'on peut insérer du code Latex ce qui est très précieux pour les documents de travail
$$E =M C^2$$
Mieux, on peut transformer le visuel en un document pdf. Il suffit de faire Export-> Chrome-> PDF et le doc pdf s'enregistre à côté du document .md.

**Mieux !**

Si j'ajoute le préambule
```Latex
---
title: "NeatWork 5 – Format des documents de travail"
author: "JPV, 8 mai 2026"
date: \today

pdf-engine: xelatex
mainfont: "Helvetica"
# ou mainfont: "Arial"
fontsize: 12pt
geometry:
  - margin=2cm
numbersections: true
#toc: true
#toc-depth: 2
vheader-includes:
  - \usepackage{setspace}
  - \setstretch{1.1}
  - \usepackage{fancyhdr}
  - \pagestyle{fancy}
  - \fancyfoot[C]{\thepage}
  - \fancyhead{}
---
```
et que je passe la commande
```JSON
pandoc UseMarkDown.md -o UseMarkDown.pdf --pdf-engine=xelatex
```
j'obtiens un super document pdf avec mise en page suivant mes instructions, sections et sous-sections numérotées, table des matières, etc. Cerise sur le gâteau : on peut également produire des documents html.


## Pourquoi MarkDown ?
s
### Lecture, écriture, et versionnage

Le format .md, c'est du texte. Il peut être versionné efficament contraiement à du pdf, du .xlsx, .docx, .jpeg, … Les commandes de section `#`, sous-section `##`, d'environnement ` ```Latex blabla``` `, listes `1. truc, 2. chose`

1. truc, 
2. chose

ou encore gras `**Gras**`  sont simples. Le visuel fourni par GitHub, Matlab et VS Code (et sans doute d'autres encore) est très acceptable. Je me suis convaincu que c'était le meilleur outil de partage de documents de travail. Il permet aussi de documenter les codes avec des README.md très riches, très complets qui sont une nécessité pour permettre la colloaboration en ***open source***. J'ai cru comprendre que MarkDown est une version simplifée et naturelle de html. Une  balise comme `<strong>texte</strong>` s'écrit **texte** en .md. Notez qu'on peut écrire aussi <strong>texte</strong> dans le fichier .md. MarkDown le comprend car c'est un enfant de html.

### Création de pdf
Un pdf n'est pas versionnable efficacement. Il encombre un dépôt GitHub. En revanche, il peut être très utile pour la communication avec l'extérieur. Comme déclaré plus haut, on peut obtenir des mises en pages bien meilleures que les visuels des éditeurs comme VS Code. On le fait à l'aide de l'utilitaire `pandoc` qu'on lance depuis une fenêtre de commande ouverte sur le dossier où se trouve le document à exporter en pdf. Je vais faire cette opération pour ce document quand j'aurai fini de le rédiger. *Just for the fun of it!*
Il faut introduire un préambulle YAML comme donné plus haut pour retrouver une bonne part de la magie de Latex.

### Création d'un doc html

On peut facilement créer un document html avec pandoc avec la simple commande

```bash
pandoc UseMarkDown.md -o UseMarkDown.html
```
Cette commande ignore le préambule Latex. Il est très souahitable de compléter la commande par des informations donnant accès au latex et de se conformer à un style de présentation souhaité. Pour cela, il faut introduire un fichier de style CSS dans le dossier. C'est ce que j'ai fait.

```bash
pandoc UseMarkDown.md -o UseMarkDown.html --standalone --mathjax --css=style.css
```

 Si le document html ne contient que du html et pas d'images, je pense qu'il est versionnable. Le principal intérêt du html à ce stade me semble être l'aide en ligne pour les futurs utilisateurs de NW5. On peut faire des documents facile d'accès et très lisibles dans les distributions futures, même pour les versions beta lorsqu'on travaillera avec le GRET.

# Accès aux outils

- **VS Code**, aller sur le site de Visual Studio. Jusqu'à présent c'est gratuit. 
- **pandoc** Faire `pandoc --version` pour voir si l'outil est sur votre bécane.
- **Homebrew** Faire `brew --version`

Si Homebrew est absent, l’installer avec :

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Puis installer Pandoc :

```bash
brew install pandoc
```

  **En résumé**, VS Code ouvre la porte à

-  Markdown + preview ;
- Pandoc ;
- HTML/CSS ;
- JSON ;
- Git intégré ;
- environnement multi-langages.

