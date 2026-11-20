# NeatWork Modern

Desktop application for importing, visualizing, editing, and simulating NeatWork hydraulic networks.

## Documentation languages

- [Complete English setup and run guide](README_EN.md)
- [Guide complet d’installation et d’exécution en français](README_FR.md)

## Architecture overview

```text
React + Vite + Electron :5173
              │
              ▼
Spring Boot REST API :8080 ─────► PostgreSQL :5432
              │
              ▼
FastAPI Python Core :8000
```

## Quick development start

The complete prerequisites, Windows commands, configuration options, API examples, production build instructions, and troubleshooting sections are available in the language-specific guides above.

### 1. Create the PostgreSQL database

```sql
CREATE DATABASE neatworkapp;
```

Default local connection:

```text
Database: neatworkapp
Username: postgres
Password: postgres
Port: 5432
```

### 2. Start Python Core

```bash
cd python-core
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e .
python -m uvicorn app.main:app --host 127.0.0.1 --port 8000 --reload
```

### 3. Start the Spring Boot backend

```bash
cd backend
export NEATWORK_WORKSPACE_ROOT="$PWD/neatwork-workspace"
export NEATWORK_PYTHON_CORE_BASE_URL='http://127.0.0.1:8000'
mvn clean spring-boot:run
```

> The Maven wrapper files are incomplete in the current archive. Use a system Maven installation unless `.mvn/wrapper` is restored.

### 4. Start React and Electron

```bash
cd frontend
npm ci
npm run dev
```

### 5. Verify services

```text
Python Core: http://127.0.0.1:8000/api/health
Backend:     http://localhost:8080/api/health
Frontend:    http://localhost:5173
```

## Important production note

The current Electron packaging configuration is not yet fully self-contained: PostgreSQL and Java are still required, the Windows Python executable path needs adaptation, and packaged-app CORS should be reviewed. See the complete English or French guide before producing a distributable installer.
