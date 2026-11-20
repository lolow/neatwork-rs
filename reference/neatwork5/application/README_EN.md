# NeatWork Modern — Complete Setup and Run Guide

This document explains how to install, configure, run, verify, and package the NeatWork desktop application on a new machine.

> The repository is composed of three services that must work together during development:
>
> 1. **Python Core** — FastAPI service for NeatWork file parsing and hydraulic simulation.
> 2. **Backend** — Spring Boot REST API, PostgreSQL persistence, workspace and artifact management.
> 3. **Frontend/Desktop** — React + Vite user interface displayed inside Electron.

---

## Table of contents

1. [Architecture](#step-1--understand-the-architecture)
2. [Prerequisites](#step-2--install-the-prerequisites)
3. [Extract or clone the project](#step-3--extract-or-clone-the-project)
4. [Create the PostgreSQL database](#step-4--create-the-postgresql-database)
5. [Run Python Core](#step-5--install-and-run-python-core)
6. [Run the Spring Boot backend](#step-6--configure-and-run-the-spring-boot-backend)
7. [Run the React/Electron frontend](#step-7--install-and-run-the-reactelectron-frontend)
8. [Verify the complete installation](#step-8--verify-the-complete-installation)
9. [Use the application for the first time](#step-9--first-use-workflow)
10. [Configuration reference](#step-10--configuration-reference)
11. [API examples](#step-11--useful-api-examples)
12. [Build production artifacts](#step-12--build-production-artifacts)
13. [Tests and quality checks](#step-13--tests-and-quality-checks)
14. [Troubleshooting](#step-14--troubleshooting)
15. [Known repository limitations](#step-15--known-repository-limitations)

---

# Step 1 — Understand the architecture

```text
┌──────────────────────────────────────────────────────────┐
│ React + Vite UI inside Electron                          │
│ Development URL: http://localhost:5173                   │
└──────────────────────────┬───────────────────────────────┘
                           │ HTTP / JSON
                           ▼
┌──────────────────────────────────────────────────────────┐
│ Spring Boot Backend                                      │
│ URL: http://localhost:8080                               │
│ - Projects                                               │
│ - Network graph                                          │
│ - File artifacts                                         │
│ - Domain snapshots                                       │
│ - PostgreSQL persistence                                 │
└───────────────┬───────────────────────┬──────────────────┘
                │                       │
                │ HTTP / JSON           │ JDBC
                ▼                       ▼
┌──────────────────────────┐  ┌────────────────────────────┐
│ Python Core / FastAPI    │  │ PostgreSQL                 │
│ http://localhost:8000    │  │ localhost:5432             │
│ - TPO/DSG/JSON parser    │  │ Database: neatworkapp      │
│ - Simulation service     │  └────────────────────────────┘
└──────────────────────────┘
```

## Main features present in the current code

- Create and list projects.
- Store a project graph in PostgreSQL.
- Display visual and raw 2D network graphs.
- Move nodes and persist their visual positions.
- Open or close valve edges.
- Upload and version project artifacts.
- Import NeatWork data from `TPO`, `DSG`, and `JSON` files.
- Optionally use diameter and orifice database files during TPO import.
- Save imported domain snapshots as PostgreSQL `jsonb` data.
- Run the current Python hydraulic simulation and save the returned pressures, flows, and velocities.
- Display domain tables and raw imported text in the frontend.

## Repository structure

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

# Step 2 — Install the prerequisites

Install the following tools before running the project.

| Tool | Required or recommended version | Why it is needed |
|---|---:|---|
| Java JDK | **21** | Required by `backend/pom.xml` |
| Maven | **3.9.x recommended** | Builds and runs the Spring Boot backend |
| PostgreSQL | A maintained local version | Stores projects, graphs, artifacts, and JSON snapshots |
| Python | **3.10 or newer** | Required by `python-core/pyproject.toml` |
| Node.js | **22.x recommended** | Runs Vite, TypeScript, React, and Electron |
| npm | Included with Node.js | Installs frontend dependencies |
| Git | Optional | Cloning and version control |
| curl | Optional | Service health checks from a terminal |

The frontend production build was verified with:

```text
Node.js 22.16.0
npm 10.9.2
```

The Python service was verified with Python 3.13.5, although the project supports Python 3.10 or newer.

## Check the installed versions

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

## Important Maven wrapper note

The archive contains `backend/mvnw` and `backend/mvnw.cmd`, but it does **not** contain the required `.mvn/wrapper/maven-wrapper.properties` directory and file. Therefore, the included Maven wrapper is currently incomplete.

Use a system Maven installation:

```bash
mvn spring-boot:run
```

Do not rely on `./mvnw` or `mvnw.cmd` unless the `.mvn/wrapper` files are restored first.

---

# Step 3 — Extract or clone the project

Extract the archive into a path without unusual permission restrictions.

Examples:

```text
macOS/Linux: /Users/your-name/projects/neatwork-app-desktop
Windows:     C:\Users\your-name\projects\neatwork-app-desktop
```

Then open a terminal in the project root:

### macOS / Linux

```bash
cd /path/to/neatwork-app-desktop
```

### Windows PowerShell

```powershell
cd C:\path\to\neatwork-app-desktop
```

You will normally use **three terminals** during development:

```text
Terminal 1 → Python Core
Terminal 2 → Spring Boot Backend
Terminal 3 → Frontend + Electron
```

---

# Step 4 — Create the PostgreSQL database

The default backend configuration is:

```properties
spring.datasource.url=jdbc:postgresql://localhost:5432/neatworkapp
spring.datasource.username=postgres
spring.datasource.password=postgres
```

## Option A — Use the default local credentials

Ensure PostgreSQL is running and that the `postgres` user has the password `postgres`. Then create the database.

### Using `createdb`

```bash
createdb -h localhost -p 5432 -U postgres neatworkapp
```

### Using `psql`

```bash
psql -h localhost -p 5432 -U postgres
```

Run this SQL:

```sql
CREATE DATABASE neatworkapp;
```

Exit:

```sql
\q
```

## Option B — Use your own PostgreSQL username and password

The recommended approach is to override the Spring Boot values with environment variables instead of editing and committing secrets.

### macOS / Linux

```bash
export SPRING_DATASOURCE_URL='jdbc:postgresql://localhost:5432/neatworkapp'
export SPRING_DATASOURCE_USERNAME='your_postgres_user'
export SPRING_DATASOURCE_PASSWORD='your_postgres_password'
```

### Windows PowerShell

```powershell
$env:SPRING_DATASOURCE_URL='jdbc:postgresql://localhost:5432/neatworkapp'
$env:SPRING_DATASOURCE_USERNAME='your_postgres_user'
$env:SPRING_DATASOURCE_PASSWORD='your_postgres_password'
```

## Database schema creation

No manual SQL migration is required for the first local run. The current configuration uses:

```properties
spring.jpa.hibernate.ddl-auto=update
```

Hibernate will create or update the required tables when the backend starts successfully.

Expected tables include:

```text
projects
network_nodes
network_edges
project_artifacts
project_domain_snapshots
```

## Verify the database

```bash
psql -h localhost -U postgres -d neatworkapp -c '\dt'
```

The table list can be empty before the first successful backend startup.

---

# Step 5 — Install and run Python Core

Open **Terminal 1**.

## 5.1 Enter the Python project

```bash
cd python-core
```

## 5.2 Create a virtual environment

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

If PowerShell blocks activation for the current session:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\.venv\Scripts\Activate.ps1
```

## 5.3 Install dependencies

```bash
python -m pip install --upgrade pip
python -m pip install -e .
```

Dependencies declared by the project include:

```text
FastAPI
Uvicorn
Pydantic
```

## 5.4 Start the FastAPI development server

```bash
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000 --reload
```

Alternative launcher without auto-reload:

```bash
python desktop_launcher.py
```

Keep this terminal open.

## 5.5 Verify Python Core

Open another terminal or browser and check:

```bash
curl http://127.0.0.1:8000/api/health
```

Expected response shape:

```json
{
  "service": "neatwork-python-core",
  "status": "UP",
  "timestamp": "..."
}
```

FastAPI interactive documentation is available at:

```text
http://127.0.0.1:8000/docs
```

Relevant Python endpoints:

```text
GET  /health
GET  /api/health
GET  /api/core/health
POST /api/neatwork/parse
POST /api/simulation/run
```

---

# Step 6 — Configure and run the Spring Boot backend

Open **Terminal 2**.

## 6.1 Enter the backend directory

```bash
cd backend
```

## 6.2 Set an explicit workspace directory

The default value is relative:

```properties
neatwork.workspace.root-dir=./neatwork-workspace
```

A relative path changes according to the directory from which Java is started. For predictable behavior, use an absolute path.

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

Set the PostgreSQL environment variables here as well when you are not using the default credentials.

## 6.3 Start the backend with Maven

```bash
mvn clean spring-boot:run
```

The backend should start on:

```text
http://localhost:8080
```

Keep this terminal open.

## 6.4 Verify the backend

```bash
curl http://localhost:8080/api/health
```

Expected response shape:

```json
{
  "service": "neatwork-backend",
  "status": "UP",
  "timestamp": "..."
}
```

Verify communication between the backend and Python Core:

```bash
curl http://localhost:8080/api/python-core/health
```

Expected status:

```json
{
  "service": "neatwork-python-core",
  "status": "UP",
  "timestamp": "..."
}
```

When PostgreSQL is empty, the backend automatically creates one seed project and a small demo graph during startup.

## 6.5 Run from a packaged JAR instead

Build:

```bash
mvn clean package -DskipTests
```

Run:

```bash
java -jar target/backend-0.0.1-SNAPSHOT.jar
```

---

# Step 7 — Install and run the React/Electron frontend

Open **Terminal 3**.

## 7.1 Enter the frontend directory

```bash
cd frontend
```

## 7.2 Install exact locked dependencies

```bash
npm ci
```

Use `npm ci` for a clean reproducible installation based on `package-lock.json`. Use `npm install` only when intentionally updating dependencies.

## 7.3 Optional API URL override

The frontend defaults to:

```text
http://localhost:8080
```

To use another backend URL, create `frontend/.env.local`:

```env
VITE_API_BASE_URL=http://localhost:8080
```

Vite environment variables are read when the frontend development server or production build starts.

## 7.4 Start Vite and Electron

```bash
npm run dev
```

This command runs both:

```text
Vite development server → http://localhost:5173
Electron desktop window  → loads http://localhost:5173
```

The script is defined as:

```json
"dev": "concurrently \"vite\" \"wait-on http://localhost:5173 && electron .\""
```

The development Electron process does **not** start the Java or Python services. Python Core and the Spring Boot backend must already be running in Terminals 1 and 2.

## Browser-only development

To run only the Vite web interface:

```bash
npx vite
```

Then open:

```text
http://localhost:5173
```

## Frontend production build check

```bash
npm run build
```

The generated static frontend is written to:

```text
frontend/dist/
```

Preview it with:

```bash
npm run preview
```

---

# Step 8 — Verify the complete installation

All four local ports should be available:

| Component | Default address |
|---|---|
| PostgreSQL | `localhost:5432` |
| Python Core | `http://127.0.0.1:8000` |
| Spring Backend | `http://localhost:8080` |
| Vite Frontend | `http://localhost:5173` |

## Health checks

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

## Expected application header

The Electron window should display:

```text
NeatWork Modern
Python powered hydraulic simulation
Backend: UP
Python Core: UP
```

If the UI was opened before the backend was ready, refresh or restart the frontend after both service health checks return `UP`.

---

# Step 9 — First-use workflow

## 9.1 Select or create a project

On the first successful backend startup, a seed project is inserted when the database has no projects.

You can also create a new project from the frontend. A newly created project currently receives the demo graph as its initial graph.

## 9.2 Upload project artifacts

Supported artifact types in the backend include:

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

The application stores input files inside the configured workspace:

```text
<workspace-root>/<project-id>/inputs/
```

Generated files are stored under:

```text
<workspace-root>/<project-id>/outputs/
```

Files of the same artifact type are versioned using names such as:

```text
network_v1.tpo
network_v2.tpo
```

## 9.3 Import a domain

Supported input formats:

```text
TPO
DSG
JSON
```

### TPO requirement

When a TPO file does not contain a `!Diameters` section, a diameter database artifact must be uploaded as `DB_DIAMETERS_INPUT` and selected during import.

An orifice database is optional. The parser can return warnings when neither an `!Orifices` section nor an orifice database is available.

### DSG requirement

A DSG file must contain at least:

```text
!Nodes
!Pipes
```

The parser can generate warnings when no `!Clients` section exists.

## 9.4 Inspect and edit the network

The frontend supports:

- Visual graph view.
- Raw 2D coordinate view.
- Domain tables.
- Raw domain text.
- Node selection and visual coordinate updates.
- Edge inspection.
- Valve open/close actions.

## 9.5 Run the simulation

Click **Run simulation**.

The backend sends the current graph to Python Core. Python Core returns:

- Node pressure.
- Low-pressure flag.
- Edge flow rate.
- Edge velocity.
- Number of open edges.
- Number of closed edges.
- Number of low-pressure nodes.

The backend saves these returned values in PostgreSQL and sends the updated graph back to the frontend.

> The current Python simulation is a simplified graph-based prototype. It should not be treated as a validated EPANET-equivalent hydraulic solver without further scientific validation.

---

# Step 10 — Configuration reference

## Backend settings

| Environment variable | Default value | Purpose |
|---|---|---|
| `SERVER_PORT` | `8080` | Spring Boot HTTP port |
| `SPRING_DATASOURCE_URL` | `jdbc:postgresql://localhost:5432/neatworkapp` | PostgreSQL JDBC URL |
| `SPRING_DATASOURCE_USERNAME` | `postgres` | PostgreSQL username |
| `SPRING_DATASOURCE_PASSWORD` | `postgres` | PostgreSQL password |
| `NEATWORK_PYTHON_CORE_BASE_URL` | `http://localhost:8000` | Python Core base URL |
| `NEATWORK_WORKSPACE_ROOT` | `./neatwork-workspace` | Artifact workspace root |

The corresponding source file is:

```text
backend/src/main/resources/application.properties
```

## Python Core settings

The default launcher binds to:

```text
Host: 127.0.0.1
Port: 8000
```

Change them directly in the Uvicorn command when necessary:

```bash
python -m uvicorn app.main:app --host 0.0.0.0 --port 8010
```

Then point the backend to the new URL:

```bash
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8010'
```

## Frontend settings

| Variable | Default value | Purpose |
|---|---|---|
| `VITE_API_BASE_URL` | `http://localhost:8080` | Spring Boot API base URL |

Example `frontend/.env.local`:

```env
VITE_API_BASE_URL=http://localhost:8080
```

## Current CORS settings

The Spring backend currently allows frontend requests from:

```text
http://localhost:5173
```

The Python Core currently allows:

```text
http://localhost:5173
http://localhost:8080
http://localhost:8000
```

When hostnames or ports change, update:

```text
backend/src/main/java/com/neatwork/backend/config/CorsConfig.java
python-core/app/main.py
```

---

# Step 11 — Useful API examples

## List projects

```bash
curl http://localhost:8080/api/projects
```

## Create a project

```bash
curl -X POST http://localhost:8080/api/projects \
  -H 'Content-Type: application/json' \
  -d '{"name":"My hydraulic network"}'
```

Save the returned project UUID for the following commands.

```bash
PROJECT_ID='replace-with-project-uuid'
```

## Read the project graph

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/graph"
```

## Upload a TPO artifact

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/artifacts" \
  -F 'type=TPO_INPUT' \
  -F 'file=@/absolute/path/network.tpo'
```

Save the returned artifact UUID:

```bash
PRIMARY_ARTIFACT_ID='replace-with-artifact-uuid'
```

## Upload a diameter database

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/artifacts" \
  -F 'type=DB_DIAMETERS_INPUT' \
  -F 'file=@/absolute/path/diameters.db'
```

## Import from uploaded artifacts

```bash
curl -X POST \
  "http://localhost:8080/api/projects/$PROJECT_ID/domain/import-from-artifacts" \
  -H 'Content-Type: application/json' \
  -d "{
    \"name\": \"Imported network\",
    \"format\": \"TPO\",
    \"primaryArtifactId\": \"$PRIMARY_ARTIFACT_ID\",
    \"diametersDbArtifactId\": null,
    \"orificesDbArtifactId\": null
  }"
```

## List project artifacts

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/artifacts"
```

## Read the latest domain snapshot

```bash
curl "http://localhost:8080/api/projects/$PROJECT_ID/domain/latest"
```

## Run a project simulation

```bash
curl -X POST "http://localhost:8080/api/projects/$PROJECT_ID/graph"
```

## Change a valve status

```bash
EDGE_ID='V1'

curl -X PATCH \
  "http://localhost:8080/api/projects/$PROJECT_ID/edges/$EDGE_ID/status" \
  -H 'Content-Type: application/json' \
  -d '{"status":"CLOSED"}'
```

## Move a node

```bash
NODE_ID='J1'

curl -X PATCH \
  "http://localhost:8080/api/projects/$PROJECT_ID/nodes/$NODE_ID/position" \
  -H 'Content-Type: application/json' \
  -d '{"x":350,"y":200}'
```

---

# Step 12 — Build production artifacts

The repository contains an Electron Builder configuration that packages:

- The compiled React frontend.
- The Spring Boot JAR.
- A PyInstaller Python executable.

Build in the following order.

## 12.1 Build the Python executable

```bash
cd python-core
```

Activate the project virtual environment, then install PyInstaller:

```bash
python -m pip install pyinstaller
```

Build from the provided specification:

```bash
pyinstaller --clean neatwork-python-core.spec
```

Expected output on macOS/Linux:

```text
python-core/dist/neatwork-python-core
```

On Windows, PyInstaller normally creates:

```text
python-core/dist/neatwork-python-core.exe
```

## 12.2 Build the Spring Boot JAR

```bash
cd ../backend
mvn clean package -DskipTests
```

Expected output:

```text
backend/target/backend-0.0.1-SNAPSHOT.jar
```

## 12.3 Build the Electron installer/package

```bash
cd ../frontend
npm ci
npm run dist
```

Electron Builder writes packages into:

```text
frontend/release/
```

## Important production packaging caveats

The current production launcher in `frontend/electron/main.cjs`:

- Starts Python from `resources/python-core/neatwork-python-core`.
- Starts Java using the `java` command from the operating system PATH.
- Starts the backend with the `desktop` Spring profile.
- Waits three seconds and then opens the Electron window.

Before distributing a production package, address the limitations listed in [Step 15](#step-15--known-repository-limitations), especially PostgreSQL, Windows executable naming, Java runtime availability, and packaged-app CORS.

---

# Step 13 — Tests and quality checks

## Backend test

The repository currently contains a Spring context-load test.

Run:

```bash
cd backend
mvn test
```

Because the test loads the full Spring context, PostgreSQL configuration may be required unless a dedicated test database profile is added.

## Frontend build and lint

```bash
cd frontend
npm run build
npx eslint .
```

The current frontend build command performs TypeScript checking before Vite compilation:

```text
tsc && vite build
```

## Frontend dependency audit

```bash
npm audit
```

Review audit results before applying an automatic major upgrade. Avoid blindly running a forced audit fix on an active project branch.

## Python syntax check

```bash
cd python-core
python -m compileall app
```

## Python manual health check

```bash
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000
curl http://127.0.0.1:8000/api/health
```

The repository does not currently include a dedicated Python test suite.

---

# Step 14 — Troubleshooting

## Problem: `mvnw` cannot read `maven-wrapper.properties`

Typical message:

```text
cannot read distributionUrl property in .mvn/wrapper/maven-wrapper.properties
```

Cause: the `.mvn/wrapper` files are missing from the archive.

Solution:

```bash
mvn spring-boot:run
```

Use a system Maven installation, or regenerate/restore the Maven wrapper files.

---

## Problem: backend cannot connect to PostgreSQL

Typical errors:

```text
Connection refused
password authentication failed
database "neatworkapp" does not exist
```

Check:

```bash
psql -h localhost -p 5432 -U postgres -d neatworkapp
```

Confirm that:

- PostgreSQL is running.
- Port `5432` is correct.
- The database exists.
- Username and password match the backend environment variables.

---

## Problem: port already in use

Check the port owner.

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

Use another port and update the dependent service URLs.

Example Python port change:

```bash
python -m uvicorn app.main:app --port 8010
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8010'
```

---

## Problem: frontend says one or more services are unreachable

Check in this order:

```bash
curl http://127.0.0.1:8000/api/health
curl http://localhost:8080/api/health
curl http://localhost:8080/api/python-core/health
```

Then restart or refresh the frontend.

The initial frontend loading logic does not continuously retry all failed startup requests.

---

## Problem: backend reports Python Core as `DOWN`

Confirm Python is listening on port `8000`:

```bash
curl http://127.0.0.1:8000/api/health
```

Confirm the backend variable:

```bash
printenv NEATWORK_PYTHON_CORE_BASE_URL
```

Windows PowerShell:

```powershell
$env:NEATWORK_PYTHON_CORE_BASE_URL
```

Expected value:

```text
http://127.0.0.1:8000
```

---

## Problem: TPO import says a diameter database is required

The selected TPO file has no exact `!Diameters` section.

Upload the diameter database using artifact type:

```text
DB_DIAMETERS_INPUT
```

Then select its artifact ID during import.

---

## Problem: DSG import fails

The DSG parser requires:

```text
!Nodes
!Pipes
```

Check section names and file encoding. Artifact text is first read as UTF-8 and then retried as ISO-8859-1.

---

## Problem: artifact exists in PostgreSQL but file is missing

Artifact metadata is stored in PostgreSQL, but the real file is stored in the workspace directory. Moving or deleting the workspace causes missing-file errors.

Use a stable absolute value for:

```text
NEATWORK_WORKSPACE_ROOT
```

Back up PostgreSQL and the workspace together.

---

## Problem: Electron opens a blank or failed page in development

Confirm Vite is running:

```text
http://localhost:5173
```

Then restart:

```bash
npm run dev
```

---

## Problem: `npm ci` fails after manually changing `package.json`

`package.json` and `package-lock.json` must be synchronized.

For an intentional dependency update:

```bash
rm -rf node_modules
npm install
```

Review and commit the resulting lockfile changes.

Windows PowerShell equivalent:

```powershell
Remove-Item -Recurse -Force node_modules
npm install
```

---

# Step 15 — Known repository limitations

These points are based on the current source code and should be considered before production distribution.

1. **The Maven wrapper is incomplete.**  
   `mvnw` and `mvnw.cmd` are present, but `.mvn/wrapper/maven-wrapper.properties` is missing.

2. **The desktop profile does not currently provide an embedded database.**  
   `application-desktop.properties` contains commented H2 settings, and the H2 dependency is commented out in `pom.xml`. Therefore, the packaged desktop backend still uses the normal PostgreSQL configuration unless the project is changed.

3. **The packaged desktop application still requires Java.**  
   Electron launches the backend with `spawn("java", ...)`. A JRE/JDK must be available on the user's PATH unless a Java runtime is bundled.

4. **The Python executable path is platform-sensitive.**  
   The launcher expects `neatwork-python-core` without a Windows `.exe` suffix. Windows packaging requires a conditional executable name or corresponding Electron Builder configuration.

5. **Packaged Electron CORS may need adjustment.**  
   The backend CORS configuration only allows `http://localhost:5173`. A packaged Electron renderer loads from a local file URL unless a custom protocol is configured.

6. **No Docker Compose file is included.**  
   PostgreSQL and all three application layers must currently be started manually during development.

7. **No database migration tool is configured.**  
   The project currently relies on `spring.jpa.hibernate.ddl-auto=update`. Flyway or Liquibase is recommended before controlled production deployment.

8. **Default database credentials are development-only.**  
   `postgres/postgres` should not be used in a shared or production environment.

9. **The current Python simulation is a simplified prototype.**  
   It calculates pressure loss and flow using application-specific heuristic rules, not a validated full hydraulic engine.

10. **Automated test coverage is limited.**  
    The backend has a context-load test, while dedicated frontend and Python test suites are not present in the archive.

---

# Recommended development startup summary

Run these commands in three terminals after PostgreSQL is ready.

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

Then verify:

```text
Python Core: http://127.0.0.1:8000/api/health
Backend:     http://localhost:8080/api/health
Frontend:    http://localhost:5173
```

---

# Safe shutdown

Stop each development service with:

```text
Ctrl + C
```

Deactivate the Python virtual environment with:

```bash
deactivate
```

PostgreSQL can remain running for the next development session.
