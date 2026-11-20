# NeatWork Modern — Guide complet d’installation et d’exécution

Ce document explique comment installer, configurer, lancer, vérifier et empaqueter l’application de bureau NeatWork sur une nouvelle machine.

> Le dépôt est composé de trois services qui doivent fonctionner ensemble pendant le développement :
>
> 1. **Python Core** — service FastAPI chargé de l’analyse des fichiers NeatWork et de la simulation hydraulique.
> 2. **Backend** — API REST Spring Boot, persistance PostgreSQL, gestion du workspace et des artefacts.
> 3. **Frontend/Desktop** — interface React + Vite affichée dans Electron.

---

## Sommaire

1. [Architecture](#étape-1--comprendre-larchitecture)
2. [Prérequis](#étape-2--installer-les-prérequis)
3. [Extraire ou cloner le projet](#étape-3--extraire-ou-cloner-le-projet)
4. [Créer la base PostgreSQL](#étape-4--créer-la-base-de-données-postgresql)
5. [Lancer Python Core](#étape-5--installer-et-lancer-python-core)
6. [Lancer le backend Spring Boot](#étape-6--configurer-et-lancer-le-backend-spring-boot)
7. [Lancer le frontend React/Electron](#étape-7--installer-et-lancer-le-frontend-reactelectron)
8. [Vérifier l’installation complète](#étape-8--vérifier-linstallation-complète)
9. [Première utilisation](#étape-9--premier-flux-dutilisation)
10. [Référence de configuration](#étape-10--référence-de-configuration)
11. [Exemples d’API](#étape-11--exemples-utiles-dapi)
12. [Créer les artefacts de production](#étape-12--créer-les-artefacts-de-production)
13. [Tests et contrôles qualité](#étape-13--tests-et-contrôles-qualité)
14. [Dépannage](#étape-14--dépannage)
15. [Limites connues du dépôt](#étape-15--limites-connues-du-dépôt)

---

# Étape 1 — Comprendre l’architecture

```text
┌──────────────────────────────────────────────────────────┐
│ Interface React + Vite dans Electron                     │
│ URL de développement : http://localhost:5173             │
└──────────────────────────┬───────────────────────────────┘
                           │ HTTP / JSON
                           ▼
┌──────────────────────────────────────────────────────────┐
│ Backend Spring Boot                                      │
│ URL : http://localhost:8080                              │
│ - Projets                                                │
│ - Graphe du réseau                                       │
│ - Fichiers et artefacts                                  │
│ - Snapshots de domaine                                   │
│ - Persistance PostgreSQL                                 │
└───────────────┬───────────────────────┬──────────────────┘
                │                       │
                │ HTTP / JSON           │ JDBC
                ▼                       ▼
┌──────────────────────────┐  ┌────────────────────────────┐
│ Python Core / FastAPI    │  │ PostgreSQL                 │
│ http://localhost:8000    │  │ localhost:5432             │
│ - Parser TPO/DSG/JSON    │  │ Base : neatworkapp         │
│ - Service de simulation  │  └────────────────────────────┘
└──────────────────────────┘
```

## Fonctionnalités principales présentes dans le code actuel

- Créer et lister des projets.
- Enregistrer le graphe d’un projet dans PostgreSQL.
- Afficher le graphe visuel et les coordonnées 2D brutes du réseau.
- Déplacer les nœuds et enregistrer leurs positions visuelles.
- Ouvrir ou fermer les arêtes de type vanne.
- Importer, stocker et versionner les artefacts d’un projet.
- Importer des données NeatWork depuis des fichiers `TPO`, `DSG` et `JSON`.
- Utiliser éventuellement des bases de diamètres et d’orifices pendant l’import TPO.
- Enregistrer les domaines importés dans PostgreSQL au format `jsonb`.
- Exécuter la simulation hydraulique Python actuelle et sauvegarder pressions, débits et vitesses.
- Afficher les tables du domaine et le contenu textuel brut importé.

## Structure du dépôt

```text
project-root/
├── backend/
│   ├── pom.xml
│   ├── mvnw / mvnw.cmd
│   ├── src/main/java/com/neatwork/backend/
│   └── src/main/resources/
│       ├── application.properties
│       └── application-desktop.properties
├── frontend/
│   ├── package.json
│   ├── package-lock.json
│   ├── vite.config.ts
│   ├── electron/main.cjs
│   └── src/
├── python-core/
│   ├── pyproject.toml
│   ├── desktop_launcher.py
│   ├── neatwork-python-core.spec
│   └── app/
└── README.md
```

---

# Étape 2 — Installer les prérequis

Installez les outils suivants avant de lancer le projet.

| Outil | Version requise ou recommandée | Utilité |
|---|---:|---|
| Java JDK | **21** | Requis par `backend/pom.xml` |
| Maven | **3.9.x recommandé** | Compile et lance le backend Spring Boot |
| PostgreSQL | Une version locale maintenue | Stocke projets, graphes, artefacts et snapshots JSON |
| Python | **3.10 ou supérieur** | Requis par `python-core/pyproject.toml` |
| Node.js | **22.x recommandé** | Exécute Vite, TypeScript, React et Electron |
| npm | Fourni avec Node.js | Installe les dépendances frontend |
| Git | Facultatif | Clonage et gestion de versions |
| curl | Facultatif | Vérification des services depuis le terminal |

Le build de production du frontend a été vérifié avec :

```text
Node.js 22.16.0
npm 10.9.2
```

Le service Python a été vérifié avec Python 3.13.5, bien que le projet accepte Python 3.10 ou supérieur.

## Vérifier les versions installées

### macOS / Linux

```bash
java -version
mvn -version
python3 --version
node --version
npm --version
psql --version
```

### Windows PowerShell

```powershell
java -version
mvn -version
python --version
node --version
npm --version
psql --version
```

## Remarque importante sur Maven Wrapper

L’archive contient `backend/mvnw` et `backend/mvnw.cmd`, mais elle ne contient pas le dossier ni le fichier requis `.mvn/wrapper/maven-wrapper.properties`. Le Maven Wrapper fourni est donc incomplet.

Utilisez une installation Maven disponible sur le système :

```bash
mvn spring-boot:run
```

N’utilisez pas `./mvnw` ou `mvnw.cmd` tant que les fichiers `.mvn/wrapper` n’ont pas été restaurés.

---

# Étape 3 — Extraire ou cloner le projet

Extrayez l’archive dans un emplacement où votre utilisateur possède les droits de lecture et d’écriture.

Exemples :

```text
macOS/Linux : /Users/votre-nom/projects/neatwork-app-desktop
Windows :     C:\Users\votre-nom\projects\neatwork-app-desktop
```

Ouvrez ensuite un terminal à la racine du projet.

### macOS / Linux

```bash
cd /chemin/vers/neatwork-app-desktop
```

### Windows PowerShell

```powershell
cd C:\chemin\vers\neatwork-app-desktop
```

En développement, utilisez normalement **trois terminaux** :

```text
Terminal 1 → Python Core
Terminal 2 → Backend Spring Boot
Terminal 3 → Frontend + Electron
```

---

# Étape 4 — Créer la base de données PostgreSQL

La configuration par défaut du backend est la suivante :

```properties
spring.datasource.url=jdbc:postgresql://localhost:5432/neatworkapp
spring.datasource.username=postgres
spring.datasource.password=postgres
```

## Option A — Utiliser les identifiants locaux par défaut

Vérifiez que PostgreSQL est démarré et que l’utilisateur `postgres` possède le mot de passe `postgres`. Créez ensuite la base.

### Avec `createdb`

```bash
createdb -h localhost -p 5432 -U postgres neatworkapp
```

### Avec `psql`

```bash
psql -h localhost -p 5432 -U postgres
```

Exécutez :

```sql
CREATE DATABASE neatworkapp;
```

Quittez :

```sql
\q
```

## Option B — Utiliser votre propre utilisateur PostgreSQL

Il est préférable de surcharger les propriétés Spring Boot avec des variables d’environnement plutôt que de modifier et committer des secrets.

### macOS / Linux

```bash
export SPRING_DATASOURCE_URL='jdbc:postgresql://localhost:5432/neatworkapp'
export SPRING_DATASOURCE_USERNAME='votre_utilisateur_postgres'
export SPRING_DATASOURCE_PASSWORD='votre_mot_de_passe_postgres'
```

### Windows PowerShell

```powershell
$env:SPRING_DATASOURCE_URL='jdbc:postgresql://localhost:5432/neatworkapp'
$env:SPRING_DATASOURCE_USERNAME='votre_utilisateur_postgres'
$env:SPRING_DATASOURCE_PASSWORD='votre_mot_de_passe_postgres'
```

## Création du schéma

Aucun script SQL manuel n’est nécessaire pour le premier lancement local. La configuration actuelle utilise :

```properties
spring.jpa.hibernate.ddl-auto=update
```

Hibernate crée ou met à jour les tables au démarrage réussi du backend.

Tables attendues :

```text
projects
network_nodes
network_edges
project_artifacts
project_domain_snapshots
```

## Vérifier la base

```bash
psql -h localhost -U postgres -d neatworkapp -c '\dt'
```

La liste peut être vide avant le premier démarrage réussi du backend.

---

# Étape 5 — Installer et lancer Python Core

Ouvrez le **Terminal 1**.

## 5.1 Entrer dans le projet Python

```bash
cd python-core
```

## 5.2 Créer un environnement virtuel

### macOS / Linux

```bash
python3 -m venv .venv
source .venv/bin/activate
```

### Windows PowerShell

```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
```

Si PowerShell bloque l’activation pour la session courante :

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\.venv\Scripts\Activate.ps1
```

## 5.3 Installer les dépendances

```bash
python -m pip install --upgrade pip
python -m pip install -e .
```

Le projet déclare notamment :

```text
FastAPI
Uvicorn
Pydantic
```

## 5.4 Démarrer le serveur FastAPI de développement

```bash
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000 --reload
```

Lanceur alternatif sans rechargement automatique :

```bash
python desktop_launcher.py
```

Gardez ce terminal ouvert.

## 5.5 Vérifier Python Core

Depuis un autre terminal ou un navigateur :

```bash
curl http://127.0.0.1:8000/api/health
```

Forme de réponse attendue :

```json
{
  "service": "neatwork-python-core",
  "status": "UP",
  "timestamp": "..."
}
```

La documentation interactive FastAPI est disponible ici :

```text
http://127.0.0.1:8000/docs
```

Endpoints Python principaux :

```text
GET  /health
GET  /api/health
GET  /api/core/health
POST /api/neatwork/parse
POST /api/simulation/run
```

---

# Étape 6 — Configurer et lancer le backend Spring Boot

Ouvrez le **Terminal 2**.

## 6.1 Entrer dans le dossier backend

```bash
cd backend
```

## 6.2 Définir un workspace explicite

La valeur par défaut est relative :

```properties
neatwork.workspace.root-dir=./neatwork-workspace
```

Un chemin relatif dépend du dossier depuis lequel Java est lancé. Pour obtenir un comportement prévisible, utilisez un chemin absolu.

### macOS / Linux

```bash
export NEATWORK_WORKSPACE_ROOT="$PWD/neatwork-workspace"
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8000'
```

### Windows PowerShell

```powershell
$env:NEATWORK_WORKSPACE_ROOT="$PWD\neatwork-workspace"
$env:NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8000'
```

Définissez également ici les variables PostgreSQL lorsque vous n’utilisez pas les identifiants par défaut.

## 6.3 Démarrer le backend avec Maven

```bash
mvn clean spring-boot:run
```

Le backend doit démarrer sur :

```text
http://localhost:8080
```

Gardez ce terminal ouvert.

## 6.4 Vérifier le backend

```bash
curl http://localhost:8080/api/health
```

Forme de réponse attendue :

```json
{
  "service": "neatwork-backend",
  "status": "UP",
  "timestamp": "..."
}
```

Vérifiez la communication entre le backend et Python Core :

```bash
curl http://localhost:8080/api/python-core/health
```

Statut attendu :

```json
{
  "service": "neatwork-python-core",
  "status": "UP",
  "timestamp": "..."
}
```

Lorsque PostgreSQL ne contient aucun projet, le backend crée automatiquement un projet initial et un petit graphe de démonstration.

## 6.5 Exécuter un JAR empaqueté

Compiler :

```bash
mvn clean package -DskipTests
```

Lancer :

```bash
java -jar target/backend-0.0.1-SNAPSHOT.jar
```

---

# Étape 7 — Installer et lancer le frontend React/Electron

Ouvrez le **Terminal 3**.

## 7.1 Entrer dans le dossier frontend

```bash
cd frontend
```

## 7.2 Installer exactement les dépendances verrouillées

```bash
npm ci
```

Utilisez `npm ci` pour une installation propre et reproductible basée sur `package-lock.json`. Utilisez `npm install` uniquement lorsque vous souhaitez mettre à jour les dépendances.

## 7.3 Surcharger éventuellement l’URL de l’API

Le frontend utilise par défaut :

```text
http://localhost:8080
```

Pour utiliser une autre URL, créez `frontend/.env.local` :

```env
VITE_API_BASE_URL=http://localhost:8080
```

Les variables Vite sont lues au démarrage du serveur de développement ou pendant le build de production.

## 7.4 Démarrer Vite et Electron

```bash
npm run dev
```

Cette commande lance :

```text
Serveur Vite          → http://localhost:5173
Fenêtre Electron      → charge http://localhost:5173
```

Le script utilisé est :

```json
"dev": "concurrently \"vite\" \"wait-on http://localhost:5173 && electron .\""
```

En mode développement, Electron ne démarre **ni Java ni Python**. Python Core et le backend Spring Boot doivent déjà tourner dans les Terminaux 1 et 2.

## Développement uniquement dans le navigateur

Pour lancer seulement l’interface Vite :

```bash
npx vite
```

Ouvrez ensuite :

```text
http://localhost:5173
```

## Vérifier le build frontend

```bash
npm run build
```

Les fichiers statiques générés sont écrits dans :

```text
frontend/dist/
```

Prévisualisez-les avec :

```bash
npm run preview
```

---

# Étape 8 — Vérifier l’installation complète

Les quatre ports locaux doivent être accessibles :

| Composant | Adresse par défaut |
|---|---|
| PostgreSQL | `localhost:5432` |
| Python Core | `http://127.0.0.1:8000` |
| Backend Spring | `http://localhost:8080` |
| Frontend Vite | `http://localhost:5173` |

## Vérifications de santé

### macOS / Linux

```bash
curl -f http://127.0.0.1:8000/api/health
curl -f http://localhost:8080/api/health
curl -f http://localhost:8080/api/python-core/health
curl -f http://localhost:5173
```

### Windows PowerShell

```powershell
Invoke-RestMethod http://127.0.0.1:8000/api/health
Invoke-RestMethod http://localhost:8080/api/health
Invoke-RestMethod http://localhost:8080/api/python-core/health
Invoke-WebRequest http://localhost:5173
```

## En-tête attendu dans l’application

La fenêtre Electron doit afficher :

```text
NeatWork Modern
Python powered hydraulic simulation
Backend: UP
Python Core: UP
```

Si l’interface a été ouverte avant que le backend soit prêt, actualisez ou relancez le frontend après avoir obtenu `UP` pour les deux services.

---

# Étape 9 — Premier flux d’utilisation

## 9.1 Sélectionner ou créer un projet

Au premier démarrage réussi, un projet initial est créé lorsque la base ne contient aucun projet.

Vous pouvez également créer un projet depuis l’interface. Dans le code actuel, chaque nouveau projet reçoit initialement le graphe de démonstration.

## 9.2 Importer des artefacts

Types d’artefacts pris en charge :

```text
TPO_INPUT
DSG_INPUT
DB_DIAMETERS_INPUT
DB_ORIFICES_INPUT
JSON_INPUT
DSG_OUTPUT
JSON_OUTPUT
OTHER
```

Les fichiers d’entrée sont stockés dans :

```text
<workspace-root>/<project-id>/inputs/
```

Les fichiers générés sont stockés dans :

```text
<workspace-root>/<project-id>/outputs/
```

Les fichiers d’un même type sont versionnés, par exemple :

```text
network_v1.tpo
network_v2.tpo
```

## 9.3 Importer un domaine

Formats acceptés :

```text
TPO
DSG
JSON
```

### Exigence TPO

Lorsqu’un fichier TPO ne contient pas de section `!Diameters`, une base de diamètres doit être importée comme `DB_DIAMETERS_INPUT` puis sélectionnée pendant l’import.

La base d’orifices est facultative. Le parser peut produire un avertissement si aucune section `!Orifices` et aucune base d’orifices ne sont disponibles.

### Exigence DSG

Un fichier DSG doit contenir au minimum :

```text
!Nodes
!Pipes
```

Le parser peut produire un avertissement lorsqu’il n’existe pas de section `!Clients`.

## 9.4 Inspecter et modifier le réseau

Le frontend permet :

- L’affichage du graphe visuel.
- L’affichage des coordonnées 2D brutes.
- L’affichage des tables du domaine.
- L’affichage du contenu textuel brut.
- La sélection d’un nœud et la modification de sa position visuelle.
- L’inspection d’une arête.
- L’ouverture ou la fermeture d’une vanne.

## 9.5 Lancer la simulation

Cliquez sur **Run simulation**.

Le backend envoie le graphe courant à Python Core. Python Core renvoie :

- La pression de chaque nœud.
- L’indicateur de basse pression.
- Le débit de chaque arête.
- La vitesse de chaque arête.
- Le nombre d’arêtes ouvertes.
- Le nombre d’arêtes fermées.
- Le nombre de nœuds à basse pression.

Le backend sauvegarde ces valeurs dans PostgreSQL puis renvoie le graphe actualisé au frontend.

> La simulation Python actuelle est un prototype simplifié basé sur le graphe. Elle ne doit pas être considérée comme équivalente à un solveur hydraulique EPANET validé sans validation scientifique supplémentaire.

---

# Étape 10 — Référence de configuration

## Paramètres du backend

| Variable d’environnement | Valeur par défaut | Rôle |
|---|---|---|
| `SERVER_PORT` | `8080` | Port HTTP Spring Boot |
| `SPRING_DATASOURCE_URL` | `jdbc:postgresql://localhost:5432/neatworkapp` | URL JDBC PostgreSQL |
| `SPRING_DATASOURCE_USERNAME` | `postgres` | Utilisateur PostgreSQL |
| `SPRING_DATASOURCE_PASSWORD` | `postgres` | Mot de passe PostgreSQL |
| `NEATWORK_PYTHON_CORE_BASE_URL` | `http://localhost:8000` | URL de base de Python Core |
| `NEATWORK_WORKSPACE_ROOT` | `./neatwork-workspace` | Racine du workspace des artefacts |

Fichier source correspondant :

```text
backend/src/main/resources/application.properties
```

## Paramètres de Python Core

Le lanceur par défaut utilise :

```text
Hôte : 127.0.0.1
Port : 8000
```

Pour changer ces valeurs :

```bash
python -m uvicorn app.main:app --host 0.0.0.0 --port 8010
```

Puis configurez le backend :

```bash
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8010'
```

## Paramètres du frontend

| Variable | Valeur par défaut | Rôle |
|---|---|---|
| `VITE_API_BASE_URL` | `http://localhost:8080` | URL de base de l’API Spring Boot |

Exemple `frontend/.env.local` :

```env
VITE_API_BASE_URL=http://localhost:8080
```

## Configuration CORS actuelle

Le backend Spring autorise actuellement les requêtes provenant de :

```text
http://localhost:5173
```

Python Core autorise :

```text
http://localhost:5173
http://localhost:8080
http://localhost:8000
```

Si les hôtes ou les ports changent, modifiez :

```text
backend/src/main/java/com/neatwork/backend/config/CorsConfig.java
python-core/app/main.py
```

---

# Étape 11 — Exemples utiles d’API

## Lister les projets

```bash
curl http://localhost:8080/api/projects
```

## Créer un projet

```bash
curl -X POST http://localhost:8080/api/projects \
  -H 'Content-Type: application/json' \
  -d '{"name":"Mon réseau hydraulique"}'
```

Conservez l’UUID retourné :

```bash
PROJECT_ID='remplacer-par-uuid-projet'
```

## Lire le graphe du projet

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/graph"
```

## Importer un artefact TPO

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/artifacts" \
  -F 'type=TPO_INPUT' \
  -F 'file=@/chemin/absolu/network.tpo'
```

Conservez l’UUID de l’artefact :

```bash
PRIMARY_ARTIFACT_ID='remplacer-par-uuid-artefact'
```

## Importer une base de diamètres

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/artifacts" \
  -F 'type=DB_DIAMETERS_INPUT' \
  -F 'file=@/chemin/absolu/diameters.db'
```

## Importer le domaine depuis les artefacts

```bash
curl -X POST \
  "http://localhost:8080/api/projects/$PROJECT_ID/domain/import-from-artifacts" \
  -H 'Content-Type: application/json' \
  -d "{
    \"name\": \"Réseau importé\",
    \"format\": \"TPO\",
    \"primaryArtifactId\": \"$PRIMARY_ARTIFACT_ID\",
    \"diametersDbArtifactId\": null,
    \"orificesDbArtifactId\": null
  }"
```

## Lister les artefacts d’un projet

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/artifacts"
```

## Lire le dernier snapshot de domaine

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/domain/latest"
```

## Lancer une simulation

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/graph"
```

## Modifier l’état d’une vanne

```bash
EDGE_ID='V1'

curl -X PATCH \
  "http://localhost:8080/api/projects/$PROJECT_ID/edges/$EDGE_ID/status" \
  -H 'Content-Type: application/json' \
  -d '{"status":"CLOSED"}'
```

## Déplacer un nœud

```bash
NODE_ID='J1'

curl -X PATCH \
  "http://localhost:8080/api/projects/$PROJECT_ID/nodes/$NODE_ID/position" \
  -H 'Content-Type: application/json' \
  -d '{"x":350,"y":200}'
```

---

# Étape 12 — Créer les artefacts de production

La configuration Electron Builder du dépôt empaquette :

- Le frontend React compilé.
- Le JAR Spring Boot.
- Un exécutable Python créé avec PyInstaller.

Respectez l’ordre suivant.

## 12.1 Créer l’exécutable Python

```bash
cd python-core
```

Activez l’environnement virtuel puis installez PyInstaller :

```bash
python -m pip install pyinstaller
```

Compilez avec le fichier fourni :

```bash
pyinstaller --clean neatwork-python-core.spec
```

Sortie attendue sous macOS/Linux :

```text
python-core/dist/neatwork-python-core
```

Sous Windows, PyInstaller produit normalement :

```text
python-core/dist/neatwork-python-core.exe
```

## 12.2 Créer le JAR Spring Boot

```bash
cd ../backend
mvn clean package -DskipTests
```

Sortie attendue :

```text
backend/target/backend-0.0.1-SNAPSHOT.jar
```

## 12.3 Créer le package Electron

```bash
cd ../frontend
npm ci
npm run dist
```

Electron Builder écrit les packages dans :

```text
frontend/release/
```

## Points importants avant une distribution

Le lanceur de production actuel dans `frontend/electron/main.cjs` :

- Démarre Python depuis `resources/python-core/neatwork-python-core`.
- Démarre Java via la commande `java` du PATH du système.
- Lance le backend avec le profil Spring `desktop`.
- Attend trois secondes avant d’ouvrir la fenêtre Electron.

Avant de distribuer le package, traitez les limites détaillées dans l’[Étape 15](#étape-15--limites-connues-du-dépôt), notamment PostgreSQL, le nom de l’exécutable Windows, la disponibilité de Java et le CORS de l’application empaquetée.

---

# Étape 13 — Tests et contrôles qualité

## Test du backend

Le dépôt contient actuellement un test de chargement du contexte Spring.

```bash
cd backend
mvn test
```

Comme ce test charge tout le contexte Spring, une configuration PostgreSQL peut être nécessaire tant qu’un profil de base de test dédié n’a pas été ajouté.

## Build et lint du frontend

```bash
cd frontend
npm run build
npx eslint .
```

La commande de build vérifie TypeScript avant la compilation Vite :

```text
tsc && vite build
```

## Audit des dépendances frontend

```bash
npm audit
```

Analysez les résultats avant d’appliquer une mise à niveau majeure automatique. Évitez un correctif forcé aveugle sur une branche active.

## Vérification syntaxique Python

```bash
cd python-core
python -m compileall app
```

## Vérification manuelle de Python Core

```bash
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000
curl http://127.0.0.1:8000/api/health
```

Le dépôt ne contient pas encore de suite de tests Python dédiée.

---

# Étape 14 — Dépannage

## Problème : `mvnw` ne trouve pas `maven-wrapper.properties`

Message typique :

```text
cannot read distributionUrl property in .mvn/wrapper/maven-wrapper.properties
```

Cause : les fichiers `.mvn/wrapper` sont absents de l’archive.

Solution :

```bash
mvn spring-boot:run
```

Utilisez Maven installé sur le système ou restaurez/régénérez les fichiers du wrapper.

---

## Problème : le backend ne se connecte pas à PostgreSQL

Erreurs fréquentes :

```text
Connection refused
password authentication failed
database "neatworkapp" does not exist
```

Testez :

```bash
psql -h localhost -p 5432 -U postgres -d neatworkapp
```

Vérifiez que :

- PostgreSQL est démarré.
- Le port `5432` est correct.
- La base existe.
- L’utilisateur et le mot de passe correspondent aux variables du backend.

---

## Problème : port déjà utilisé

### macOS / Linux

```bash
lsof -i :8000
lsof -i :8080
lsof -i :5173
```

### Windows PowerShell

```powershell
Get-NetTCPConnection -LocalPort 8000
Get-NetTCPConnection -LocalPort 8080
Get-NetTCPConnection -LocalPort 5173
```

Utilisez un autre port puis mettez à jour les URLs des services dépendants.

Exemple :

```bash
python -m uvicorn app.main:app --port 8010
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8010'
```

---

## Problème : le frontend indique qu’un ou plusieurs services sont inaccessibles

Vérifiez dans cet ordre :

```bash
curl http://127.0.0.1:8000/api/health
curl http://localhost:8080/api/health
curl http://localhost:8080/api/python-core/health
```

Actualisez ensuite le frontend ou relancez-le.

Le chargement initial du frontend ne réessaie pas continuellement toutes les requêtes échouées.

---

## Problème : le backend indique que Python Core est `DOWN`

Vérifiez Python :

```bash
curl http://127.0.0.1:8000/api/health
```

Vérifiez la variable du backend :

```bash
printenv NEATWORK_PYTHON_CORE_BASE_URL
```

Sous Windows PowerShell :

```powershell
$env:NEATWORK_PYTHON_CORE_BASE_URL
```

Valeur attendue :

```text
http://127.0.0.1:8000
```

---

## Problème : l’import TPO exige une base de diamètres

Le fichier TPO sélectionné ne contient pas de section exacte `!Diameters`.

Importez la base de diamètres avec le type :

```text
DB_DIAMETERS_INPUT
```

Puis sélectionnez son identifiant pendant l’import du domaine.

---

## Problème : l’import DSG échoue

Le parser DSG exige :

```text
!Nodes
!Pipes
```

Vérifiez les noms de sections et l’encodage. Le backend lit d’abord l’artefact en UTF-8, puis réessaie en ISO-8859-1.

---

## Problème : l’artefact existe en base mais le fichier est introuvable

Les métadonnées sont stockées dans PostgreSQL, mais le vrai fichier se trouve dans le workspace. Déplacer ou supprimer le workspace provoque des erreurs de fichier absent.

Définissez une valeur absolue et stable pour :

```text
NEATWORK_WORKSPACE_ROOT
```

Sauvegardez PostgreSQL et le workspace ensemble.

---

## Problème : Electron affiche une page vide en développement

Vérifiez Vite :

```text
http://localhost:5173
```

Puis relancez :

```bash
npm run dev
```

---

## Problème : `npm ci` échoue après modification manuelle de `package.json`

`package.json` et `package-lock.json` doivent être synchronisés.

Pour une mise à jour intentionnelle :

```bash
rm -rf node_modules
npm install
```

Examinez et committez les changements du lockfile.

Équivalent PowerShell :

```powershell
Remove-Item -Recurse -Force node_modules
npm install
```

---

# Étape 15 — Limites connues du dépôt

Ces points proviennent du code actuel et doivent être traités avant une distribution en production.

1. **Le Maven Wrapper est incomplet.**  
   `mvnw` et `mvnw.cmd` existent, mais `.mvn/wrapper/maven-wrapper.properties` est absent.

2. **Le profil desktop ne fournit pas actuellement de base embarquée.**  
   `application-desktop.properties` contient des paramètres H2 commentés et la dépendance H2 est commentée dans `pom.xml`. Le backend empaqueté continue donc d’utiliser PostgreSQL tant que le projet n’est pas modifié.

3. **L’application desktop empaquetée nécessite encore Java.**  
   Electron lance le backend avec `spawn("java", ...)`. Un JRE/JDK doit être disponible dans le PATH, sauf si un runtime Java est empaqueté.

4. **Le chemin de l’exécutable Python dépend du système.**  
   Le lanceur attend `neatwork-python-core` sans suffixe `.exe`. Le package Windows nécessite un nom conditionnel ou une configuration Electron Builder adaptée.

5. **Le CORS de l’application Electron empaquetée peut nécessiter une adaptation.**  
   Le backend n’autorise que `http://localhost:5173`, alors qu’un renderer Electron empaqueté charge normalement une URL locale de type fichier sans protocole personnalisé.

6. **Aucun fichier Docker Compose n’est fourni.**  
   PostgreSQL et les trois couches applicatives doivent être démarrés manuellement en développement.

7. **Aucun outil de migration de base n’est configuré.**  
   Le projet s’appuie sur `spring.jpa.hibernate.ddl-auto=update`. Flyway ou Liquibase est recommandé avant un déploiement de production contrôlé.

8. **Les identifiants par défaut sont réservés au développement.**  
   `postgres/postgres` ne doit pas être utilisé dans un environnement partagé ou de production.

9. **La simulation Python actuelle est un prototype simplifié.**  
   Elle calcule les pertes de pression et les débits avec des règles heuristiques propres à l’application, et non avec un moteur hydraulique complet validé.

10. **La couverture de tests automatiques est limitée.**  
    Le backend possède un test de chargement du contexte. L’archive ne contient pas de suite de tests frontend ou Python dédiée.

---

# Résumé recommandé du démarrage en développement

Lorsque PostgreSQL est prêt, exécutez les commandes suivantes dans trois terminaux.

## Terminal 1 — Python Core

```bash
cd python-core
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e .
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000 --reload
```

## Terminal 2 — Backend

```bash
cd backend
export NEATWORK_WORKSPACE_ROOT="$PWD/neatwork-workspace"
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8000'
mvn clean spring-boot:run
```

## Terminal 3 — Frontend + Electron

```bash
cd frontend
npm ci
npm run dev
```

Vérifiez ensuite :

```text
Python Core : http://127.0.0.1:8000/api/health
Backend :     http://localhost:8080/api/health
Frontend :    http://localhost:5173
```

---

# Arrêt propre

Arrêtez chaque service de développement avec :

```text
Ctrl + C
```

Désactivez l’environnement Python avec :

```bash
deactivate
```

PostgreSQL peut rester démarré pour la prochaine session de développement.
