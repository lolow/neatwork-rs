from datetime import datetime, timezone

from fastapi import FastAPI, HTTPException, Request, Response
from fastapi.middleware.cors import CORSMiddleware


from fastapi.responses import JSONResponse

from app.schemas.simulation_execution import SimulationExecutionRequest,SimulationExecutionResponse

from app.services.simulation_service import export_hydraulic_statistics_json, run_hydraulic_simulation

from app.neatwork.simulation import (
    ScenarioGenerationError,
    ScenarioPreparationError,
    SimulationValidationError,
    SolverNumericalError,
    StatisticsProcessingError,
    SimulationCancelledError,
)

from app.services.simulation_service import run_hydraulic_simulation

from app.neatwork.schemas import NeatworkParseRequest, NeatworkParseResponse
from app.services.import_service import parse_neatwork
from app.services.simulation_cancellation import (
    clear_execution,
    register_execution,
    request_cancel,
)
from app.schemas.simulation_export import SimulationSpreadsheetRequest
from app.services.simulation_service import export_statistics_spreadsheet
from app.neatwork.schemas import (
    NeatworkRevisionSerializeRequest,
    NeatworkRevisionSerializeResponse,
)

from app.neatwork.serialization.revision_serializer import (serialize_revision)

app = FastAPI(
    title="Neatwork Python Core",
    version="0.0.1",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://localhost:8080", "http://localhost:8000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"]
)

# Exception handlers
@app.exception_handler(SimulationValidationError)
@app.exception_handler(ScenarioGenerationError)
@app.exception_handler(ScenarioPreparationError)
@app.exception_handler(SolverNumericalError)
@app.exception_handler(StatisticsProcessingError)
async def simulation_error_handler(
    request: Request,
    exc: Exception,
) -> JSONResponse:
    """Return structured simulation failures as HTTP 422."""
    
    if hasattr(exc, "to_dict"):
        detail = exc.to_dict()
    else:
        detail = {
            "code": "SIMULATION_FAILED",
            "message": str(exc),
        }
    
    return JSONResponse(
        status_code=422,
        content={
            "detail": detail,
        },
    )

@app.get("/health")
def root_health() -> dict:
    return health()

@app.get("/api/health")
def health() -> dict:
    return {
        "service": "neatwork-python-core",
        "status": "UP",
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

@app.get("/api/core/health")
def api_health() -> dict:
    return health()

@app.post("/api/simulation/run", response_model=SimulationExecutionResponse)
def run_simulation(request: SimulationExecutionRequest) -> SimulationExecutionResponse:
    """Execute the complete hydraulic simulation pipeline."""
    execution_id = request.execution_id
    if execution_id is not None:
        register_execution(execution_id)

    try:
        return run_hydraulic_simulation(request)
    finally:
        if execution_id is not None:
            clear_execution(execution_id)
    return run_hydraulic_simulation(request)

@app.post("/api/simulation/export")
def export_simulation(request: SimulationExecutionRequest) -> SimulationExecutionResponse:
    """Export the complete hydraulic simulation results.
        return the statistics document as a JSON file
    """
    content, filename = export_hydraulic_statistics_json(request)
    return Response(
        content=content,
        media_type="application/json",
        headers={
            "Content-Disposition": f"attachment; filename={filename}",
        },
    )

@app.post("/api/neatwork/parse", response_model=NeatworkParseResponse)
def parse_neatwork_file(request: NeatworkParseRequest) -> NeatworkParseResponse:
    try:
        return parse_neatwork(request)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

@app.exception_handler(SimulationCancelledError)
async def simulation_cancelled_handler(request: Request, exc: SimulationCancelledError) -> JSONResponse:
    """Return cancellation as HTTP 409."""
    return JSONResponse(status_code=409, content={"detail": exc.to_dict()})

@app.post("/api/simulation/executions/{execution_id}/cancel")
def cancel_simulation(execution_id: str) -> dict[str, object]:
    """Request cancellation of one running simulation."""
    requested = request_cancel(execution_id)

    return {
        "executionId": execution_id,
        "cancelRequested": requested,
    }

@app.post("/api/simulation/exports/xlsx")
def export_simulation_xlsx(request:SimulationSpreadsheetRequest) -> Response:
    """Convert canonical simulation statistics into XLSX."""

    content = export_statistics_spreadsheet(request.statistics)

    return Response(content=content,
        media_type=(
            "application/vnd.openxmlformats-officedocument. spreadsheetml.sheet"
        ),
    )

@app.post("/api/neatwork/revisions/serialize",response_model=NeatworkRevisionSerializeResponse)
def serialize_neatwork_revision(request:NeatworkRevisionSerializeRequest) -> NeatworkRevisionSerializeResponse:
    """
    Convert an edited canonical domain back to its
    original NeatWork source format and validate it
    by parsing the generated document again.
    """
    try:
        return serialize_revision(request)

    except ValueError as exc:
        raise HTTPException(status_code=400,detail=str(exc),) from exc