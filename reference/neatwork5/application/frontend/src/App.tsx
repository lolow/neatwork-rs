import axios from "axios";
import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { httpClient } from './api/httpClient';
import { fetchPythonCoreHealth } from "./api/pythonCoreApi";
import { StartScreen } from "./features/startup/StartScreen";
import { WaterIntroTransition } from "./features/startup/WaterIntroTransition";
import { NetworkCanvas } from './features/network/NetworkCanvas';
import type { NetworkGraph, EdgeStatus, NetworkSelection } from './features/network/types';
import {
  createProject,
  deleteProject,
  fetchProjects,
  cancelSimulationJob,
  fetchSimulationJob,
  fetchActiveSimulationJobs,
  startSimulationJob,
  fetchExportAvailability,
  exportProject,
  updateEdgeStatus,
  updateNodePosition,
  fetchSimulationRuns,
  fetchSimulationRun,
  deleteSimulationRun,
  fetchSimulationHistory
} from "./api/projectApi";
import type { ProjectSummary } from './features/projects/types';

import {
  fetchProjectArtifacts,
  artifactDownloadUrl,
  fetchArtifactLibrary,
  copyArtifactToProject,
  uploadProjectArtifact,
  fetchTpoArtifactRequirements,
  archiveDatabaseArtifact,
  deleteProjectInputArtifact,
} from "./api/artifactApi";
import { TpoDatabaseModal } from "./features/import/TpoDatabaseModal";
import type { TpoArtifactRequirements } from "./features/import/types";
import type { ProjectArtifact, ProjectArtifactType } from "./features/artifacts/types";

import { importDomainFromArtifacts } from "./api/domainApi";
import type { DomainSnapshot, NeatworkFileFormat } from "./features/domain/types";

import { DomainDataViewer } from './features/domain/DomainDataViewer';
import { DomainTextViewer } from "./features/domain/DomainTextViewer";

import { SimulationPanel } from "./features/simulation/SimulationPanel";

import type {
  SimulationRunOptions,
  SimulationRunRequestOptions,
  SimulationStatisticsPayload,
  SimulationSummary,
  SimulationRunDetails,
  SimulationRunSummary
} from "./features/simulation/types";

import { LinearNetworkView } from "./features/network/LinearNetworkView";
import { NetworkNodeSearch } from "./features/network/NetworkNodeSearch";
import type { NodeFocusRequest } from "./features/network/types";
import { buildSimulationDefaults } from "./features/simulation/simulationDefaults";
import { ExportControl } from "./features/export/ExportControl";
import type { ProjectExportAvailability, ProjectExportFormat } from "./features/export/types";

import type { ComparisonDataset, ComparisonSourceDescriptor, } from "./features/comparison/types";
import {
  buildDefaultComparisonDataset,
  buildDefaultComparisonSource,
  buildSimulationComparisonDataset,
  buildSimulationComparisonSource,
  getComparisonInputLabel,
} from "./features/comparison/comparisonDatasets";
import type { ComparisonWorkspaceState } from "./features/comparison/types";
import {
  clearComparisonWorkspace,
  loadComparisonWorkspace,
  saveComparisonWorkspace,
} from "./features/comparison/comparisonPersistence";
import { AppMenuBar } from "./features/menu/AppMenuBar";
import { NewProjectModal } from "./features/projects/NewProjectModal";
import { OpenProjectModal } from "./features/projects/OpenProjectModal";
import { DeleteProjectConfirmModal } from "./features/projects/DeleteProjectConfirmModal";
import { InputTypeModal } from "./features/import/InputTypeModal";
import type { NetworkInputFormat } from "./features/import/InputTypeModal";
import { InputFileLibraryModal } from "./features/import/InputFileLibraryModal";
import { WorkspaceEmptyState } from "./features/network/WorkspaceEmptyState";
import { DatabaseManagerModal } from "./features/database/DatabaseManagerModal";
import type { DatabaseType } from "./features/menu/AppMenuBar";
import { DeleteInputArtifactModal } from "./features/artifacts/DeleteInputArtifactModal";
import { saveDomainRevision } from "./api/domainApi";
import type { RevisionNamingMode } from "./features/domain/types";
import type { SimulationHistoryItem } from "./features/simulation/types";
import { SimulationHistoryDialog } from "./features/simulation/SimulationHistoryDialog";
import { DeleteSimulationDialog } from "./features/simulation/DeleteSimulationDialog";
import { SimulationDetailsDialog } from "./features/simulation/SimulationDetailsDialog";
import { updateProjectDisplayUnit } from "./api/projectApi";
import { UnitSettingsModal } from "./features/settings/UnitSettingsModal";
import type { DisplayLengthUnit } from "./features/domain/unitSystem";

import { SimulationReviewDialog } from "./features/simulation/SimulationReviewDialog";
import type { SimulationReviewMode } from "./features/simulation/SimulationReviewDialog";


function getApiErrorMessage(error: unknown, fallback: string): string {
  if (!axios.isAxiosError(error)) {
    return fallback;
  }

  const responseData = error.response?.data;

  if (responseData && typeof responseData === "object") {
    const payload = responseData as {
      message?: unknown;
      detail?: unknown;
    };

    if (typeof payload.message === "string") {
      return payload.message;
    }

    if (typeof payload.detail === "string") {
      return payload.detail;
    }

    if (payload.detail && typeof payload.detail === "object" && "message" in payload.detail) {
      const message = (payload.detail as { message?: unknown; }).message;

      if (typeof message === "string") {
        return message;
      }
    }
  }

  return fallback;
}

function isCompletedSimulationRun(run: SimulationRunSummary): boolean {
  return (
    run.status === "COMPLETED"
    ||
    run.status === "COMPLETED_WITH_ERRORS"
  );
}

type AppPhase = "start" | "transition" | "workspace";

export default function App() {

  // Application flow
  const [appPhase, setAppPhase] = useState<AppPhase>("start");

  // Core state
  const [graph, setGraph] = useState<NetworkGraph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [projectsLoading, setProjectsLoading] = useState(false);
  const [projectLoadError, setProjectLoadError] = useState<string | null>(null);

  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [newProjectModalOpen, setNewProjectModalOpen] = useState(false);
  const [openProjectModalOpen, setOpenProjectModalOpen] = useState(false);
  const [projectPendingDeletion, setProjectPendingDeletion] = useState<ProjectSummary | null>(null);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const [inputTypeModalOpen, setInputTypeModalOpen] = useState(false);
  const [inputLibraryModalOpen, setInputLibraryModalOpen] = useState(false);
  const [selectedInputFormat, setSelectedInputFormat] = useState<NetworkInputFormat | null>(null);
  const [inputLibraryArtifacts, setInputLibraryArtifacts] = useState<ProjectArtifact[]>([]);
  const [inputLibraryLoading, setInputLibraryLoading] = useState(false);
  const [inputLibraryError, setInputLibraryError] = useState<string | null>(null);
  const [importingLibraryArtifactId, setImportingLibraryArtifactId] = useState<string | null>(null);

  const [selection, setSelection] = useState<NetworkSelection>(null);
  const [simulationSummary, setSimulationSammary] = useState<SimulationSummary | null>(null);
  const [simulationStatistics, setSimulationStatistics] = useState<SimulationStatisticsPayload | null>(null);
  const [activeSimulationJobIds, setActiveSimulationJobIds] = useState<string[]>([]);
  const [foregroundSimulationJobId, setForegroundSimulationJobId] = useState<string | null>(null);
  const [simulationCancelling, setSimulationCancelling] = useState(false);
  const requestedSimulationOptionsRef = useRef<Map<string, SimulationRunRequestOptions>>(new Map());
  const simulationRunning = activeSimulationJobIds.length > 0;
  const activeSimulationCount = activeSimulationJobIds.length;
  const [simulationExporting, setSimulationExporting] = useState(false);
  const [lastSimulationOptions, setLastSimulationOptions] = useState<SimulationRunRequestOptions | null>(null);
  const [artifacts, setArtifacts] = useState<ProjectArtifact[]>([]);
  const [latestDomainSnapshot, setLatestDomainSnapshot] = useState<DomainSnapshot | null>(null);
  const [domainImported, setDomainImported] = useState(false);
  const [networkViewMode, setNetworkViewMode] = useState<"visualGraph" | "rawGraph" | "linearGraph" | "tables" | "text">("visualGraph");
  const [nodeFocusRequest, setNodeFocusRequest] = useState<NodeFocusRequest | null>(null);
  const rawGraph = buildRawGraph(graph);
  const hasImportedDomain = domainImported && graph !== null && latestDomainSnapshot !== null;
  const searchableGraph = networkViewMode === "rawGraph" ? rawGraph : graph;
  const simulationDefaults = useMemo(() => buildSimulationDefaults(latestDomainSnapshot?.domain), [latestDomainSnapshot?.domain]);
  const [exportAvailability, setExportAvailability] = useState<ProjectExportAvailability[]>([]);

  const [simulationRuns, setSimulationRuns] = useState<SimulationRunSummary[]>([]);
  const [simulationReviewOpen, setSimulationReviewOpen] = useState(false);
  const [simulationReviewMode, setSimulationReviewMode] = useState<SimulationReviewMode>("SINGLE");
  const [simulationReviewRunId, setSimulationReviewRunId] = useState<string | null>(null);

  const simulationRunCacheRef = useRef<Map<string, SimulationRunDetails>>(new Map());

  const [comparisonWorkspace, setComparisonWorkspace] = useState<ComparisonWorkspaceState | null>(null);

  const [uploadingInputFile, setUploadingInputFile] = useState(false);
  const [tpoDatabaseModalOpen, setTpoDatabaseModalOpen] = useState(false);
  const [pendingTpoArtifact, setPendingTpoArtifact] = useState<ProjectArtifact | null>(null);
  const [tpoRequirements, setTpoRequirements] = useState<TpoArtifactRequirements | null>(null);
  const [tpoRequirementsLoading, setTpoRequirementsLoading] = useState(false);
  const [diametersLibraryArtifacts, setDiametersLibraryArtifacts] = useState<ProjectArtifact[]>([]);
  const [orificesLibraryArtifacts, setOrificesLibraryArtifacts] = useState<ProjectArtifact[]>([]);
  const [tpoConfigError, setTpoConfigError] = useState<string | null>(null);
  const [tpoImporting, setTpoImporting] = useState(false);

  const [activeDatabaseType, setActiveDatabaseType] = useState<DatabaseType | null>(null);
  const [databaseArtifacts, setDatabaseArtifacts] = useState<ProjectArtifact[]>([]);
  const [databaseLoading, setDatabaseLoading] = useState(false);
  const [databaseUploading, setDatabaseUploading] = useState(false);
  const [databaseError, setDatabaseError] = useState<string | null>(null);

  const [pendingArtifactDeletion, setPendingArtifactDeletion] = useState<ProjectArtifact | null>(null);
  const [deletingArtifactId, setDeletingArtifactId] = useState<string | null>(null);
  const [artifactDeleteError, setArtifactDeleteError] = useState<string | null>(null);
  const [simulationHistoryOpen, setSimulationHistoryOpen] = useState(false);
  const [simulationHistory, setSimulationHistory] = useState<SimulationHistoryItem[]>([]);
  const [simulationHistoryLoading, setSimulationHistoryLoading] = useState(false);
  const [simulationHistoryError, setSimulationHistoryError] = useState<string | null>(null);
  const [historyCancellingRunId, setHistoryCancellingRunId] = useState<string | null>(null);
  const [pendingSimulationDeletion, setPendingSimulationDeletion] = useState<SimulationHistoryItem | null>(null);
  const [deletingSimulationId, setDeletingSimulationId] = useState<string | null>(null);
  const [simulationDeleteError, setSimulationDeleteError] = useState<string | null>(null);

  const [simulationDetailsOpen, setSimulationDetailsOpen] = useState(false);
  const [simulationDetails, setSimulationDetails] = useState<SimulationRunDetails | null>(null);
  const [simulationDetailsLoading, setSimulationDetailsLoading] = useState(false);
  const [simulationDetailsError, setSimulationDetailsError] = useState<string | null>(null);

  const [simulationResultDetails, setSimulationResultDetails] = useState<SimulationRunDetails | null>(null);
  const [simulationResultLoading, setSimulationResultLoading] = useState(false);
  const [simulationResultError, setSimulationResultError] = useState<string | null>(null);
  // Completed runs waiting for their result window to be displayed.
  const automaticResultQueueRef = useRef<string[]>([]);
  // Prevents the same completed run from being automatically announced more than once.
  const announcedResultIdsRef = useRef<Set<string>>(new Set());
  // Prevents automatic result notifications from replacing a Review workspace currently in use.
  const reviewDialogBusyRef = useRef(false);

  const [unitSettingsOpen, setUnitSettingsOpen] = useState(false);
  const [unitSettingsSaving, setUnitSettingsSaving] = useState(false);
  const [unitSettingsError, setUnitSettingsError] = useState<string | null>(null);

  const [graphViewMenuOpen, setGraphViewMenuOpen] = useState(false);
  const graphViewMenuRef = useRef<HTMLDivElement | null>(null);

  // Loads one persisted completed run into the Single simulation section of Hydraulic Review.
  const loadSimulationReviewRun = useCallback(
    async (projectId: string, runId: string) => {
      setSimulationReviewRunId(runId);
      setSimulationResultDetails(null);
      setSimulationResultError(null);
      setSimulationResultLoading(true);
      const cached = simulationRunCacheRef.current.get(runId);
      if (cached) {
        if (!cached.statistics) {
          setSimulationResultError("This simulation does not contain result tables.");
        } else {
          setSimulationResultDetails(cached);
        }
        setSimulationResultLoading(false);
        return;
      }

      try {
        const details = await fetchSimulationRun(projectId, runId);
        simulationRunCacheRef.current.set(runId, details);
        if (!details.statistics) {
          setSimulationResultError("This simulation does not contain result tables.");
          return;
        }

        setSimulationResultDetails(details);
      } catch (error) {
        console.error("Could not load simulation review:", error);
        setSimulationResultError(getApiErrorMessage(error, "Could not load simulation results."));
      } finally {
        setSimulationResultLoading(false);
      }
    }, []
  );

  // Opens Hydraulic Review directly on one simulation.
  // This path is used by automatic completion and simulation-history result actions.
  const openSimulationReviewSingle = useCallback(
    async (projectId: string, runId: string) => {

      reviewDialogBusyRef.current = true;
      setSimulationReviewMode("SINGLE");
      setSimulationReviewOpen(true);
      await loadSimulationReviewRun(projectId, runId);
    },
    [loadSimulationReviewRun]
  );

  // Opens the next automatically queued result only when no other result window currently owns the UI.
  const openNextQueuedSimulationResult = useCallback((projectId: string) => {
    if (reviewDialogBusyRef.current) {
      return;
    }

    const nextRunId = automaticResultQueueRef.current.shift();

    if (!nextRunId) {
      return;
    }
    void openSimulationReviewSingle(projectId, nextRunId);
  },
    [openSimulationReviewSingle]
  );

  // Schedules one newly completed simulation result for automatic display.
  const enqueueAutomaticSimulationResult = useCallback((projectId: string, runId: string) => {

    if (announcedResultIdsRef.current.has(runId)) {
      return;
    }
    announcedResultIdsRef.current.add(runId);
    automaticResultQueueRef.current.push(runId);
    openNextQueuedSimulationResult(projectId);
  },
    [openNextQueuedSimulationResult]
  );

  // Refreshes the complete persistent simulation lifecycle for the selected project.
  const refreshSimulationHistory = useCallback(
    async (projectId: string) => {
      const history = await fetchSimulationHistory(projectId);
      setSimulationHistory(history);
      return history;
    },
    []
  );

  const activeSourceArtifact = useMemo(() => {
    const sourceId = latestDomainSnapshot?.primaryArtifactId;
    if (!sourceId) {
      return null;
    }
    return artifacts.find(artifact => artifact.id === sourceId) ?? null;
  },
    [artifacts, latestDomainSnapshot?.primaryArtifactId]
  );

  const comparisonRuns =
    useMemo(
      () => {

        if (!latestDomainSnapshot) {
          return [];
        }

        const currentSnapshotId =
          latestDomainSnapshot.id;

        return simulationRuns

          .filter(
            isCompletedSimulationRun
          )

          .sort(
            (
              left,
              right
            ) => {

              const leftCurrent =
                left.domainSnapshotId
                ===
                currentSnapshotId;

              const rightCurrent =
                right.domainSnapshotId
                ===
                currentSnapshotId;


              if (
                leftCurrent
                !==
                rightCurrent
              ) {

                return leftCurrent
                  ? -1
                  : 1;
              }


              return (
                right.runNumber
                -
                left.runNumber
              );
            }
          );
      },
      [
        simulationRuns,
        latestDomainSnapshot?.id,
      ]
    );

  const comparisonInputLabel = useMemo(() => {
    if (!latestDomainSnapshot) {
      return "";
    }
    return getComparisonInputLabel(latestDomainSnapshot);
  },
    [latestDomainSnapshot]);

  const comparisonSources = useMemo<ComparisonSourceDescriptor[]>(() => {
    if (!latestDomainSnapshot) {
      return [];
    }
    const sources: ComparisonSourceDescriptor[] = [buildDefaultComparisonSource(latestDomainSnapshot)];
    comparisonRuns.forEach(run => {
      sources.push(buildSimulationComparisonSource(run, comparisonInputLabel, run.runNumber));
    }
    );
    return sources;
  },
    [latestDomainSnapshot, comparisonRuns, comparisonInputLabel]
  );

  const reviewComparisonSources = useMemo(() => comparisonSources.filter(
    source => source.sourceType === "SIMULATION")
    , [comparisonSources]
  );

  const selectedProject = useMemo(() =>
    projects.find(project => project.id === selectedProjectId) ?? null,
    [projects, selectedProjectId]
  );

  // Single source of truth for every display-unit decision in the current project.
  const projectDisplayUnit: DisplayLengthUnit = selectedProject?.displayUnit ?? "m";

  // Determines which network format can be imported.
  // The committed project format has priority. Before the first import, staged input artifacts also restrict the available format.
  const stagedDsg = artifacts.some(artifact => artifact.type === "DSG_INPUT");
  const stagedTpo = artifacts.some(artifact => artifact.type === "TPO_INPUT");
  const importBlocked = selectedProject?.sourceFormat === "REVIEW_REQUIRED" ||
    (selectedProject?.sourceFormat === "NONE" && stagedDsg && stagedTpo);

  const allowedImportFormat: NetworkInputFormat | null = selectedProject?.sourceFormat === "DSG"
    ? "DSG" : selectedProject?.sourceFormat === "TPO"
      ? "TPO" : stagedDsg && !stagedTpo
        ? "DSG" : stagedTpo && !stagedDsg
          ? "TPO" : null;

  const loadComparisonDataset = useCallback(async (sourceId: string): Promise<ComparisonDataset> => {
    if (sourceId.startsWith("default:")) {
      if (!latestDomainSnapshot) {
        throw new Error("No imported domain is available.");
      }
      return buildDefaultComparisonDataset(latestDomainSnapshot);
    }
    if (sourceId.startsWith("simulation:")) {
      if (!selectedProjectId) {
        throw new Error("No project is selected.");
      }
      const runId = sourceId.slice("simulation:".length);
      const allowedRun = comparisonRuns.find(run => run.id === runId);
      if (!allowedRun) {
        throw new Error("This simulation cannot be loaded for comparison.");
      }
      const cached = simulationRunCacheRef.current.get(runId);
      if (cached) {

        return buildSimulationComparisonDataset(cached, comparisonInputLabel, cached.runNumber);
      }
      const details = await fetchSimulationRun(selectedProjectId, runId);
      if (details.projectId !== selectedProjectId) {
        throw new Error("This simulation belongs to another project.");
      }

      simulationRunCacheRef.current.set(runId, details);
      return buildSimulationComparisonDataset(details, comparisonInputLabel, details.runNumber);
    }
    throw new Error(`Unknown comparison source: ${sourceId}`);
  },
    [latestDomainSnapshot, selectedProjectId, comparisonRuns, comparisonInputLabel]
  );

  const handleComparisonWorkspaceChange = useCallback(
    (workspace: ComparisonWorkspaceState) => {
      setComparisonWorkspace(workspace);
      if (selectedProjectId && latestDomainSnapshot) {
        saveComparisonWorkspace(selectedProjectId, latestDomainSnapshot.id, workspace);
      }
    },
    [selectedProjectId, latestDomainSnapshot]
  );

  const graphViewActive = networkViewMode === "visualGraph" ||
    networkViewMode === "rawGraph" ||
    networkViewMode === "linearGraph";

  // Load initial data
  useEffect(() => {
    loadInitialData();
  }, []);

  // Handle project change
  useEffect(() => {
    setInputTypeModalOpen(false);
    setInputLibraryModalOpen(false);

    setSelectedInputFormat(null);

    setInputLibraryArtifacts([]);
    setInputLibraryError(null);

    setTpoDatabaseModalOpen(false);
    setPendingTpoArtifact(null);
    setTpoRequirements(null);
    setTpoRequirementsLoading(false);
    setTpoConfigError(null);
    setTpoImporting(false);
    setDiametersLibraryArtifacts([]);
    setOrificesLibraryArtifacts([]);
    setUploadingInputFile(false);
    setImportingLibraryArtifactId(null);

    setLatestDomainSnapshot(null);
    setDomainImported(false);
    setSelection(null);
    setSimulationStatistics(null);
    setSimulationSammary(null);
    setLastSimulationOptions(null);
    setArtifacts([]);

    setSimulationRuns([]);
    setSimulationHistory([]);
    setSimulationDetailsOpen(false);
    setSimulationDetails(null);
    setSimulationDetailsLoading(false);
    setSimulationDetailsError(null);
    setSimulationHistoryOpen(false);
    setSimulationHistoryLoading(false);
    setSimulationHistoryError(null);
    setHistoryCancellingRunId(null);
    setPendingSimulationDeletion(null);
    setDeletingSimulationId(null);
    setSimulationDeleteError(null);
    setGraphViewMenuOpen(false);

    setComparisonWorkspace(null);

    automaticResultQueueRef.current = [];
    announcedResultIdsRef.current.clear();
    reviewDialogBusyRef.current = false;
    setSimulationResultDetails(null);
    setSimulationResultError(null);
    setSimulationReviewOpen(false);
    setSimulationReviewMode("SINGLE");
    setSimulationReviewRunId(null);
    setSimulationResultLoading(false);

    simulationRunCacheRef.current.clear();

    if (!selectedProjectId) return;


    fetchProjectArtifacts(selectedProjectId)
      .then((loadedArtifacts) => {
        setArtifacts(loadedArtifacts);
        setError(null);
      })
      .catch((error) => {
        console.error("Failed to load project artifacts: ", error);
        setError("Could not load project files.");
      });
    void refreshSimulationHistory(selectedProjectId);
    fetchActiveSimulationJobs(selectedProjectId)
      .then(jobs => {
        const ids = jobs.map(job => job.jobId);
        setActiveSimulationJobIds(ids);
        // Use the newest unfinished job as the foreground execution after reconnecting to a project.
        setForegroundSimulationJobId(ids.length > 0 ? ids[ids.length - 1] : null);
      })
      .catch(error => {
        console.error("Could not load active simulations:", error);
        setActiveSimulationJobIds([]);
        setForegroundSimulationJobId(null);
      });

  }, [selectedProjectId]);

  // Load comparison workspace if project is selected
  useEffect(() => {
    if (!selectedProjectId || !latestDomainSnapshot) {
      setComparisonWorkspace(null);
      return;
    }
    setComparisonWorkspace(loadComparisonWorkspace(selectedProjectId, latestDomainSnapshot.id));
  }, [selectedProjectId, latestDomainSnapshot?.id]);

  async function refreshProjects(): Promise<void> {
    setProjectsLoading(true);
    setProjectLoadError(null);

    try {
      const projectResponse = await fetchProjects();
      setProjects(projectResponse);

      setError(current =>
        current?.startsWith("Could not load projects") ||
          current?.startsWith("Cannot connect to the backend")
          ? null : current
      );

    } catch (error) {
      console.error("Failed to load projects:", error);

      let message = "Could not load projects.";

      if (axios.isAxiosError(error)) {
        if (error.response) {
          message = `Could not load projects (HTTP ${error.response.status}). ` +
            getApiErrorMessage(error, "Check the backend logs.");
        } else {
          message = "Cannot connect to the backend. Check localhost:8080.";
        }
      }
      setProjectLoadError(message);
      setError(message);

      throw error;
    } finally {
      setProjectsLoading(false);
    }
  }

  // Load initial data - Called once on application load
  async function loadInitialData() {
    const [projectResult, backendResult, pythonResult] = await Promise.allSettled([
      refreshProjects(), httpClient.get("/api/health"), fetchPythonCoreHealth()]);

    if (projectResult.status === "rejected") {
      return;
    }

    if (backendResult.status === "rejected") {
      setError("Backend health check failed, but projects are available.");
    } else if (pythonResult.status === "rejected") {
      setError("Python Core is unavailable. Projects can still be opened.");
    } else {
      setError(null);
    }
  }

  async function handleCreateProject(name: string) {
    const created = await createProject(name);
    const nextProjects = await fetchProjects();

    setProjects(nextProjects);
    setGraph(null);
    setLatestDomainSnapshot(null);
    setDomainImported(false);
    setSelection(null);
    setSimulationStatistics(null);
    setSimulationSammary(null);
    setLastSimulationOptions(null);
    setSelectedProjectId(created.id);
  }

  async function handleCreateProjectFromModal(name: string) {
    setError(null);

    try {
      await handleCreateProject(name);
      setNewProjectModalOpen(false);
    } catch (error) {
      console.error("Project creation failed:", error);
      setError(getApiErrorMessage(error, "Could not create project."));

      throw error;
    }
  }

  async function handleConfirmDeleteProject() {
    const project = projectPendingDeletion;

    if (!project || deletingProjectId !== null) {
      return;
    }
    setDeletingProjectId(project.id);
    setError(null);

    try {
      await deleteProject(project.id);
      /*
       * Comparison configuration belongs
       * to the deleted project as well.
       */
      clearComparisonWorkspace(project.id);
      /*
       * Refresh the project list after
       * successful deletion.
       */
      const nextProjects = await fetchProjects();
      setProjects(nextProjects);
      /*
       * Close the confirmation modal.
       */
      setProjectPendingDeletion(null);
      /*
       * If another project was deleted,
       * the current workspace may remain
       * unchanged.
       */
      if (selectedProjectId !== project.id) {
        return;
      }
      /*
       * The active project itself has been
       * deleted, therefore all project-owned
       * UI state must be cleared.
       */
      setSelectedProjectId(null);
      setGraph(null);
      setLatestDomainSnapshot(null);
      setDomainImported(false);
      setArtifacts([]);
      setSelection(null);
      setSimulationStatistics(null);
      setSimulationSammary(null);
      setLastSimulationOptions(null);
      setSimulationRuns([]);
      setSimulationReviewOpen(false);
      reviewDialogBusyRef.current = false;
      setComparisonWorkspace(null);
      simulationRunCacheRef.current.clear();
    } catch (error) {
      console.error("Project deletion failed:", error);
      setError(getApiErrorMessage(error, "Could not delete project."));
    } finally {
      setDeletingProjectId(null);
    }
  }

  async function handleOpenSimulationHistory() {
    if (!selectedProjectId) {
      return;
    }
    setSimulationHistoryOpen(true);
    setSimulationHistoryLoading(true);
    setSimulationHistoryError(null);
    try {
      await refreshSimulationHistory(selectedProjectId);
    } catch (error) {
      console.error("Could not load simulation history:", error);
      setSimulationHistoryError(getApiErrorMessage(error, "Could not load simulation history."));
    } finally {
      setSimulationHistoryLoading(false);
    }
  }

  // Requests cancellation for one active history row.
  async function handleCancelHistorySimulation(run: SimulationHistoryItem) {
    if (!selectedProjectId || historyCancellingRunId !== null) {
      return;
    }

    setHistoryCancellingRunId(run.id);
    setSimulationHistoryError(null);

    try {
      await cancelSimulationJob(selectedProjectId, run.id);

      // Reflect the persisted cancellation request immediately while the background worker completes cooperative cancellation.
      setSimulationHistory(
        current => current.map(item => item.id === run.id
          ? {
            ...item,
            status: "CANCEL_REQUESTED",
            cancelRequested: true,
          }
          : item
        )
      );

      if (foregroundSimulationJobId === run.id) {
        setSimulationCancelling(true);
      }
    } catch (error) {
      console.error("Simulation cancellation failed:", error);
      setSimulationHistoryError(getApiErrorMessage(error, "Could not cancel this simulation."));
    } finally {
      setHistoryCancellingRunId(null);
    }
  }

  // Persists edited domain data as a new immutable source-file revision and activates its snapshot.
  async function handleSaveDomainRevision(domain: unknown, namingMode: RevisionNamingMode, customFilename?: string | null): Promise<ProjectArtifact> {

    if (!selectedProjectId || !latestDomainSnapshot) {
      throw new Error("No active imported domain.");
    }
    setError(null);
    let response;
    try {
      response = await saveDomainRevision({
        projectId: selectedProjectId,
        sourceSnapshotId: latestDomainSnapshot.id,
        domain,
        namingMode,
        customFilename: customFilename ?? null,
      });
    } catch (error) {
      throw new Error(getApiErrorMessage(error, "The edited revision could not be saved."));
    }

    /*
     * The newly saved revision immediately becomes the active working domain.
     */
    setLatestDomainSnapshot(response.snapshot);
    setGraph(response.graph);
    setDomainImported(true);
    setSelection(null);
    /*
     * Existing visible simulation results belong to
     * the previous input revision and must not be
     * presented as results of the edited domain.
     */
    setSimulationStatistics(null);
    setSimulationSammary(null);
    setLastSimulationOptions(null);
    setSimulationReviewOpen(false);
    reviewDialogBusyRef.current = false;
    const nextArtifacts = await fetchProjectArtifacts(selectedProjectId);
    setArtifacts(nextArtifacts);
    return response.artifact;
  }

  /**
 * Starts one asynchronous simulation using only the runtime overrides supplied by the caller.
 * Passing an empty object means that Python Core must resolve the complete configuration from the active domain.
 */
  async function handleRunSimulation(options: SimulationRunRequestOptions) {
    if (!selectedProjectId || !latestDomainSnapshot) return false;
    setError(null);
    setSimulationCancelling(false);

    try {
      const job = await startSimulationJob(selectedProjectId, options);

      // Make the new PENDING/RUNNING execution immediately visible in persistent history.
      void refreshSimulationHistoryForComparison(selectedProjectId).catch(error => {
        console.error("Could not preload simulation history:", error);
      });

      requestedSimulationOptionsRef.current.set(job.jobId, { ...options });
      setActiveSimulationJobIds(current =>
        current.includes(job.jobId) ? current : [...current, job.jobId]);

      // The newest submitted execution becomes the foreground job while older jobs keep running.
      setForegroundSimulationJobId(job.jobId);
      setSimulationSammary(null);
      setSimulationStatistics(null);
      setSelection(null);

      return true;
    } catch (error) {
      console.error("Simulation start failed: ", error);

      setError(getApiErrorMessage(error, "Could not start hydraulic simulation."));

      return false;
    }
  }

  // Starts one simulation without runtime overrides.
  // The backend and Python Core resolve all effective parameters from the active domain configuration.
  async function handleRunDefaultSimulation(): Promise<boolean> {
    return handleRunSimulation({});
  }

  useEffect(() => {
    if (!selectedProjectId || activeSimulationJobIds.length === 0) {
      return;
    }

    let disposed = false;
    let timer: number | undefined;

    async function pollJobs() {
      try {
        const jobs = await Promise.all(activeSimulationJobIds.map(jobId => fetchSimulationJob(selectedProjectId!, jobId)))

        if (disposed) return;

        // Keep visible lifecycle rows synchronized without reloading the complete history on every poll.
        setSimulationHistory(
          current => current.map(
            historyRun => {
              const liveJob = jobs.find(job => job.jobId === historyRun.id);
              if (!liveJob) {
                return historyRun;
              }
              return {
                ...historyRun,
                status: liveJob.status,
                cancelRequested: liveJob.cancelRequested,
                completedAt: liveJob.completedAt,
              };
            }
          )
        );

        const terminalIds = new Set<string>();
        let historyChanged = false;

        for (const job of jobs) {
          const terminal =
            job.status === "COMPLETED" ||
            job.status === "COMPLETED_WITH_ERRORS" ||
            job.status === "CANCELLED" ||
            job.status === "FAILED";

          const hasResult =
            job.status === "COMPLETED" || job.status === "COMPLETED_WITH_ERRORS";

          if (hasResult && job.result) {
            // Every newly completed simulation receives its own result window. 
            // Concurrent completions are serialized only at the presentation layer.
            enqueueAutomaticSimulationResult(selectedProjectId!, job.jobId);
          }
          if (!terminal)
            continue;

          terminalIds.add(job.jobId);
          historyChanged = true;
          const requested = requestedSimulationOptionsRef.current.get(job.jobId);
          /*
           * Only the foreground completion updates the main canvas and summary panel.
           * Other executions remain persisted and available in simulation history.
           */
          if (job.jobId === foregroundSimulationJobId) {
            if ((job.status === "COMPLETED" ||
              job.status === "COMPLETED_WITH_ERRORS") && job.result) {
              setGraph(job.result.graph);
              setSimulationSammary(job.result.summary);
              setSimulationStatistics(job.result.statistics);

              /*
               * Preserve the original launch request exactly.
               * {} remains {}, which means "run from defaults".
               * Manual overrides remain exactly as entered.
               */
              if (requested) {
                setLastSimulationOptions({ ...requested });
              }
              setSelection(null);
            } else if (job.status == "FAILED") {
              setError(job.errorMessage ?? "Hydraulic simulation failed.");
            }
            setForegroundSimulationJobId(null);
            setSimulationCancelling(false);
          }

          requestedSimulationOptionsRef.current.delete(job.jobId);
        }

        if (terminalIds.size > 0) {
          setActiveSimulationJobIds(current =>
            current.filter(id => !terminalIds.has(id)));
        }

        if (historyChanged && selectedProjectId) {
          await Promise.all([
            refreshSimulationHistory(selectedProjectId),
            refreshSimulationHistoryForComparison(selectedProjectId),
            refreshExportAvailability(selectedProjectId)
          ]);
        }

        const stillRunning = jobs.some(job => !terminalIds.has(job.jobId));
        if (stillRunning) {
          // PENDING, RUNNING or CANCEL_REQUESTED
          timer = window.setTimeout(pollJobs, 700);
        }
      } catch (error) {
        if (disposed) return;
        console.error("Could not read simulation job:", error);
        setError(getApiErrorMessage(error, "Could not monitor hydraulic simulation."));
        timer = window.setTimeout(pollJobs, 1500);
      }
    }

    void pollJobs();

    return () => {
      disposed = true;

      if (timer !== undefined) {
        window.clearTimeout(timer);
      }
    };
  }, [selectedProjectId, activeSimulationJobIds, foregroundSimulationJobId, requestedSimulationOptionsRef]);


  async function handleRunSimulationAgain() {
    if (!lastSimulationOptions) {
      return;
    }

    const options = stripLegacyDemandOverrides(lastSimulationOptions);

    await handleRunSimulation(options);
  }

  async function handleCancelSimulation() {
    if (!selectedProjectId || !foregroundSimulationJobId || simulationCancelling) {
      return;
    }

    setSimulationCancelling(true);

    try {
      await cancelSimulationJob(selectedProjectId, foregroundSimulationJobId);
    } catch (error) {
      console.error("Simulation cancellation failed:", error);

      setSimulationCancelling(false);

      setError(getApiErrorMessage(error, "Could not cancel simulation."));
    }
  }


  async function refreshSimulationHistoryForComparison(projectId: string) {

    try {
      const runs = await fetchSimulationRuns(projectId);
      setSimulationRuns(runs);
    } catch (error) {
      console.error("Could not load simulation history:", error);
      setSimulationRuns([]);
    }
  }

  async function handleToggleEdgeStatus(edgeId: string, nextStatus: EdgeStatus) {
    if (simulationRunning) {
      setError("Cancel the running simulation before changing network state.");
      return;
    }

    if (!selectedProjectId || !graph) return;

    const updatedEdge = await updateEdgeStatus(selectedProjectId, edgeId, nextStatus);

    setGraph({
      ...graph,
      edges: graph.edges.map((edge) => (edge.id === updatedEdge.id ? updatedEdge : edge))
    });

    setSelection({ kind: "edge", item: updatedEdge });
    setSimulationSammary(null);
    setSimulationStatistics(null);
    setLastSimulationOptions(null);
  }

  async function handleMoveNode(nodeId: string, x: number, y: number) {
    if (!selectedProjectId || !graph) return;

    const updatedNode = await updateNodePosition(selectedProjectId, nodeId, x, y);

    setGraph({
      ...graph,
      nodes: graph.nodes.map((node) => (node.id === updatedNode.id ? updatedNode : node))
    });

    setSelection({ kind: "node", item: updatedNode });
  }

  async function handleOpenDatabase(type: DatabaseType) {
    setActiveDatabaseType(type);
    setDatabaseArtifacts([]);
    setDatabaseError(null);

    setDatabaseLoading(true);

    try {
      const result = await fetchArtifactLibrary(type);
      setDatabaseArtifacts(result);

    } catch (error) {
      setDatabaseError(getApiErrorMessage(error, "Could not load database files."));
    } finally {
      setDatabaseLoading(false);
    }
  }

  async function handleUploadDatabaseFile(file: File) {
    if (!activeDatabaseType || !selectedProjectId) {
      setDatabaseError("Open a project before uploading a database.");
      return;
    }
    if (!file.name.toLowerCase().endsWith(".db")) {
      setDatabaseError("Please select a .db file.");
      return;
    }
    setDatabaseUploading(true);
    setDatabaseError(null);
    try {
      const uploaded = await uploadProjectArtifact(selectedProjectId, activeDatabaseType, file);
      setDatabaseArtifacts(current => [uploaded, ...current]);
      const nextArtifacts = await fetchProjectArtifacts(selectedProjectId);
      setArtifacts(nextArtifacts);

    } catch (error) {
      setDatabaseError(getApiErrorMessage(error, "Database upload failed."));
    } finally {
      setDatabaseUploading(false);
    }
  }

  async function handleArchiveDatabase(artifact: ProjectArtifact) {
    try {
      await archiveDatabaseArtifact(artifact.id);
      setDatabaseArtifacts(current => current.filter(item => item.id !== artifact.id));
      setDatabaseError(null);

    } catch (error) {
      setDatabaseError(getApiErrorMessage(error, "Could not remove database from Library."));

      throw error;
    }
  }

  type DomainImportResult = {
    success: boolean;
    message?: string;
  };

  async function performDomainImport(
    params: {
      name: string;
      format: NeatworkFileFormat;
      primaryArtifactId: string;
      diametersDbArtifactId?: string | null;
      orificesDbArtifactId?: string | null;
    }
  ): Promise<DomainImportResult> {
    if (simulationRunning) {
      const message = "Cancel the running simulation before importing a new domain.";
      setError(message);
      return { success: false, message };
    }

    if (!selectedProjectId) {
      const message = "Select a project before importing a network.";
      setError(message);
      return { success: false, message };
    }

    setError(null);

    setDomainImported(false);
    setGraph(null);
    setLatestDomainSnapshot(null);
    setSelection(null);

    setSimulationStatistics(null);
    setSimulationSammary(null);
    setLastSimulationOptions(null);

    /*
     * Refresh the active source format after a successful
     * import so the UI immediately reflects the restriction.
     */
    setProjects(current =>
      current.map(project =>
        project.id === selectedProjectId
          ? { ...project, sourceFormat: params.format as "DSG" | "TPO" }
          : project
      )
    );

    try {
      const response = await importDomainFromArtifacts({
        projectId: selectedProjectId,
        ...params,
      });
      setGraph(response.graph);
      setLatestDomainSnapshot(response.snapshot);
      setDomainImported(true);
      setSimulationReviewOpen(false);
      reviewDialogBusyRef.current = false;
      setNetworkViewMode("visualGraph");
      return { success: true };

    } catch (error) {
      const message = getApiErrorMessage(error, "Could not import network domain.");

      console.error("Failed to import domain:", error);

      setDomainImported(false);
      setGraph(null);
      setLatestDomainSnapshot(null);
      setSelection(null);
      setError(message);
      return { success: false, message };
    }
  }


  async function refreshExportAvailability(projectId: string) {
    try {
      const result = await fetchExportAvailability(projectId);
      setExportAvailability(result);
    } catch (error) {
      console.error("Could not load export availability:", error);
      setExportAvailability([]);
    }
  }

  async function handleProjectExport(format: ProjectExportFormat) {

    if (!selectedProjectId || simulationExporting) {
      return;
    }
    setSimulationExporting(true);
    setError(null);
    try {
      const artifact = await exportProject(selectedProjectId, format);
      const nextArtifacts = await fetchProjectArtifacts(selectedProjectId);
      setArtifacts(nextArtifacts);
      await refreshExportAvailability(selectedProjectId);

      /*
       * Start the existing artifact download flow immediately.
       */
      window.location.href = artifactDownloadUrl(selectedProjectId, artifact.id);
    } catch (error) {
      console.error("Export failed:", error);
      setError(getApiErrorMessage(error, "Could not export project."));
    } finally {
      setSimulationExporting(false);
    }
  }

  async function handleSelectInputFormat(format: NetworkInputFormat) {
    if (!selectedProjectId) {
      return;
    }
    setSelectedInputFormat(format);

    setInputTypeModalOpen(false);
    setInputLibraryModalOpen(true);

    setInputLibraryArtifacts([]);
    setInputLibraryError(null);
    setInputLibraryLoading(true);

    if (importBlocked || (allowedImportFormat !== null && allowedImportFormat !== format)) {
      setInputLibraryError("This input format is not allowed for the selected project.");
      return;
    }

    const artifactType: ProjectArtifactType = format === "DSG" ? "DSG_INPUT" : "TPO_INPUT";

    try {
      const availableArtifacts = await fetchArtifactLibrary(artifactType);
      setInputLibraryArtifacts(availableArtifacts);
    } catch (error) {
      console.error("Could not load input artifact library:", error);
      setInputLibraryError(`Could not load ${format} files.`);

    } finally {
      setInputLibraryLoading(false);
    }
  }

  async function prepareTpoImport(artifact: ProjectArtifact) {
    setPendingTpoArtifact(artifact);

    setTpoRequirements(null);

    setDiametersLibraryArtifacts([]);
    setOrificesLibraryArtifacts([]);

    setTpoConfigError(null);
    // Close every previous input-selection window before opening TPO database configuration.
    setInputTypeModalOpen(false);

    // Close the file list and show thenext step immediately.
    setInputLibraryModalOpen(false);
    setTpoDatabaseModalOpen(true);
    setTpoRequirementsLoading(true);


    try {
      const [requirements, diameters, orifices] = await Promise.all([
        fetchTpoArtifactRequirements(artifact.id),
        fetchArtifactLibrary("DB_DIAMETERS_INPUT"),
        fetchArtifactLibrary("DB_ORIFICES_INPUT"),
      ]);

      setTpoRequirements(requirements);
      setDiametersLibraryArtifacts(diameters);
      setOrificesLibraryArtifacts(orifices);
    } catch (error) {
      console.error("Could not prepare TPO import:", error);
      setTpoConfigError(getApiErrorMessage(error, "Could not analyse the selected TPO file."));
    } finally {
      setTpoRequirementsLoading(false);
    }
  }

  // Imports one already-registered network artifact using an explicit format.
  // Passing the format directly prevents React state timing from affecting drag-and-drop imports.
  async function importNetworkArtifactForFormat(artifact: ProjectArtifact, format: NetworkInputFormat) {
    if (!selectedProjectId || importingLibraryArtifactId) {
      return;
    }
    if (format === "TPO") {
      setSelectedInputFormat("TPO");
      setInputTypeModalOpen(false);
      await prepareTpoImport(artifact);

      return;
    }


    /*
     * DSG can be imported immediately.
     */
    setImportingLibraryArtifactId(
      artifact.id
    );


    setInputLibraryError(
      null
    );

    setError(
      null
    );


    try {

      const projectArtifact =
        await copyArtifactToProject(
          selectedProjectId,
          artifact.id
        );


      const nextArtifacts =
        await fetchProjectArtifacts(
          selectedProjectId
        );


      setArtifacts(
        nextArtifacts
      );


      const result =
        await performDomainImport({

          name:
            networkNameFromFilename(
              artifact.originalFilename
            ),

          format:
            "DSG",

          primaryArtifactId:
            projectArtifact.id,

          diametersDbArtifactId:
            null,

          orificesDbArtifactId:
            null,
        });


      if (!result.success) {

        setInputLibraryError(
          result.message
          ??
          "The DSG file could not be imported."
        );

        return;
      }


      handleCloseInputWorkflow();


    } catch (error) {

      console.error(
        "DSG import failed:",
        error
      );


      setInputLibraryError(
        getApiErrorMessage(
          error,
          "Could not import the selected DSG file."
        )
      );


    } finally {

      setImportingLibraryArtifactId(
        null
      );
    }
  }

  async function handleImportLibraryArtifact(artifact: ProjectArtifact) {
    if (!selectedInputFormat) {
      return;
    }

    await importNetworkArtifactForFormat(artifact, selectedInputFormat);
  }

  //Imports a dropped TPO immediately when all required diameter data is already embedded in the file.
  // TPO files missing !Diameters continue through the complementary-database configuration workflow.
  async function handleQuickTpoImport(artifact: ProjectArtifact) {
    if (!selectedProjectId) {
      return;
    }

    setInputLibraryError(null);
    try {
      const requirements = await fetchTpoArtifactRequirements(artifact.id);
      if (requirements.diametersDatabaseRequired) {

        // The user must provide diameters.db before this TPO can become an active domain.
        await prepareTpoImport(artifact);

        return;
      }
      setInputTypeModalOpen(false);
      const result = await performDomainImport({
        name: networkNameFromFilename(artifact.originalFilename),
        format: "TPO",
        primaryArtifactId: artifact.id,
        diametersDbArtifactId: null,
        orificesDbArtifactId: null,
      });

      if (!result.success) {
        setInputLibraryError(result.message ?? "The TPO file could not be imported.");

        // Keep the import workflow accessible when automatic import fails.
        setInputTypeModalOpen(true);
        return;
      }
      handleCloseInputWorkflow();
    } catch (error) {
      console.error("Quick TPO import failed:", error);
      setInputLibraryError(getApiErrorMessage(error, "Could not import the dropped TPO file."));
      setInputTypeModalOpen(true);
    }
  }

  async function handleUploadNetworkInput(file: File, format: NetworkInputFormat, quickImport = false) {
    if (!selectedProjectId || uploadingInputFile) {
      return;
    }
    const formatError = validateNetworkImportFormat(format);
    if (formatError) {
      setInputLibraryError(formatError);
      return;
    }
    const extension = format === "DSG" ? ".dsg" : ".tpo";
    if (!file.name.toLowerCase().endsWith(extension)) {
      setInputLibraryError(`Please select a ${extension} file.`);
      return;
    }
    const artifactType: ProjectArtifactType = format === "DSG" ? "DSG_INPUT" : "TPO_INPUT";
    // Keep the UI state synchronized with the explicit format, but do not depend on this state for the current operation.

    setSelectedInputFormat(format);
    setUploadingInputFile(true);
    setInputLibraryError(null);
    setError(null);

    try {
      const uploaded = await uploadProjectArtifact(selectedProjectId, artifactType, file);
      // Refresh the project's artifact list.
      const nextProjectArtifacts = await fetchProjectArtifacts(selectedProjectId);
      setArtifacts(nextProjectArtifacts);

      // Keep the library synchronized when this upload originated from the existing-file window.
      setInputLibraryArtifacts(
        current => [uploaded,
          ...current.filter(item => item.id !== uploaded.id),
        ]
      );

      if (quickImport && format === "TPO") {
        /*
         * Use the uploaded file immediately.
         * DSG -> direct import
         * TPO -> configuration step
         */
        await handleQuickTpoImport(uploaded);

        return;
      }
      // Import for "Existing File" workflow.
      await importNetworkArtifactForFormat(uploaded, format);

    } catch (error) {
      console.error("Input upload failed:", error);
      setInputLibraryError(getApiErrorMessage(error, `Could not upload the ${format} file.`));

    } finally {
      setUploadingInputFile(false);
    }
  }

  // Handles a file dropped directly on the initial Import Network dialog.
  async function handleDropNetworkInput(file: File) {
    const format = networkInputFormatFromFilename(file.name);
    if (!format) {
      setInputLibraryError("Unsupported network file. Drop a .dsg or .tpo file.");
      return;
    }
    const formatError = validateNetworkImportFormat(format);

    if (formatError) {
      setInputLibraryError(formatError);
      return;
    }

    await handleUploadNetworkInput(file, format, true);
  }

  async function handleUploadTpoDatabase(type: "DB_DIAMETERS_INPUT" | "DB_ORIFICES_INPUT", file: File): Promise<ProjectArtifact> {

    if (!selectedProjectId) {
      throw new Error("No project selected.");
    }


    if (!file.name.toLowerCase().endsWith(".db")) {
      const message = "Please select a .db file.";
      setTpoConfigError(message);
      throw new Error(message);
    }

    setTpoConfigError(null);
    try {
      const uploaded = await uploadProjectArtifact(selectedProjectId, type, file);
      const nextArtifacts = await fetchProjectArtifacts(selectedProjectId);
      setArtifacts(nextArtifacts);
      if (type === "DB_DIAMETERS_INPUT") {
        setDiametersLibraryArtifacts(current => [uploaded, ...current.filter(
          item => item.id !== uploaded.id)]);

      } else {
        setOrificesLibraryArtifacts(current => [uploaded, ...current.filter(
          item => item.id !== uploaded.id)]);
      }

      return uploaded;

    } catch (error) {
      setTpoConfigError(
        getApiErrorMessage(error, "Could not upload database file."));

      throw error;
    }
  }

  async function handleConfirmTpoImport(
    params: {
      diametersArtifactId:
      string | null;
      orificesArtifactId:
      string | null;
    }
  ) {
    if (!selectedProjectId || !pendingTpoArtifact || !tpoRequirements) {
      return;
    }


    if (tpoRequirements.diametersDatabaseRequired && !params.diametersArtifactId) {
      setTpoConfigError("This TPO file has no !Diameters section. Select or upload a diameters database.");
      return;
    }

    setTpoImporting(true);
    setTpoConfigError(null);
    setError(null);
    try {

      /*
       * 1. Copy/use TPO inside active project.
       */
      const projectTpo = await copyArtifactToProject(selectedProjectId, pendingTpoArtifact.id);

      /*
       * 2. Copy/use diameters DB.
       */
      let projectDiametersId: string | null = null;
      if (params.diametersArtifactId) {
        const diameterArtifact = diametersLibraryArtifacts.find(
          artifact => artifact.id === params.diametersArtifactId);

        if (!diameterArtifact) {
          throw new Error("Selected diameters database could not be found.");
        }

        const copied = await copyArtifactToProject(selectedProjectId, diameterArtifact.id);
        projectDiametersId = copied.id;
      }

      /*
       * 3. Copy/use orifices DB.
       */
      let projectOrificesId: string | null = null;

      if (params.orificesArtifactId) {
        const orificeArtifact = orificesLibraryArtifacts.find(
          artifact => artifact.id === params.orificesArtifactId);

        if (!orificeArtifact) {
          throw new Error("Selected orifices database could not be found.");
        }

        const copied = await copyArtifactToProject(selectedProjectId, orificeArtifact.id);
        projectOrificesId = copied.id;
      }

      /*
       * 4. Refresh Project files.
       */
      const nextArtifacts = await fetchProjectArtifacts(selectedProjectId);
      setArtifacts(nextArtifacts);

      /*
       * 5. Use the already existing domain
       *    import pipeline.
       */
      const result = await performDomainImport({
        name: networkNameFromFilename(pendingTpoArtifact.originalFilename),
        format: "TPO",
        primaryArtifactId: projectTpo.id,
        diametersDbArtifactId: projectDiametersId,
        orificesDbArtifactId: projectOrificesId,
      });

      if (!result.success) {
        setTpoConfigError(result.message ?? "The TPO file could not be imported.");
        return;
      }

      /*
       * 6. Everything succeeded.
       */
      handleCloseInputWorkflow();

    } catch (error) {
      console.error("TPO import failed:", error);

      setTpoConfigError(
        getApiErrorMessage(error, error instanceof Error
          ? error.message
          : "Could not import TPO file."
        )
      );

    } finally {
      setTpoImporting(false);
    }
  }

  // Permanently deletes one terminal simulation without modifying its source project data.
  async function handleConfirmDeleteSimulation() {
    const run = pendingSimulationDeletion;
    if (!run || !selectedProjectId || deletingSimulationId !== null) {
      return;
    }

    setDeletingSimulationId(run.id);
    setSimulationDeleteError(null);

    if (simulationReviewRunId === run.id) {
      setSimulationReviewOpen(false);
      setSimulationReviewRunId(null);
      setSimulationResultDetails(null);
      reviewDialogBusyRef.current = false;
    }
    try {
      await deleteSimulationRun(selectedProjectId, run.id);
      setSimulationHistory(
        current => current.filter(item => item.id !== run.id)
      );

      setSimulationRuns(
        current => current.filter(item => item.id !== run.id)
      );

      // Historical comparison state may reference the deleted run. Drop cached data rather than exposing a stale simulation.
      simulationRunCacheRef.current.delete(run.id);
      clearComparisonWorkspace(selectedProjectId);
      setComparisonWorkspace(null);
      setSimulationReviewOpen(false);
      reviewDialogBusyRef.current = false;
      setPendingSimulationDeletion(null);

      // Refresh authoritative completed-run and export state after deletion.
      await Promise.all([
        refreshSimulationHistory(selectedProjectId),
        refreshSimulationHistoryForComparison(selectedProjectId),
        refreshExportAvailability(selectedProjectId),
      ]);
    } catch (error) {
      console.error("Simulation deletion failed:", error);
      setSimulationDeleteError(getApiErrorMessage(error, "Could not delete the selected simulation."));
    } finally {
      setDeletingSimulationId(null);
    }
  }

  // Loads complete reproducibility metadata for one historical or active simulation execution.
  async function handleViewSimulationDetails(run: SimulationHistoryItem) {
    if (!selectedProjectId) {
      return;
    }

    setSimulationDetailsOpen(true);
    setSimulationDetails(null);
    setSimulationDetailsError(null);
    setSimulationDetailsLoading(true);

    try {
      const details = await fetchSimulationRun(selectedProjectId, run.id);
      setSimulationDetails(details);
    } catch (error) {
      console.error("Could not load simulation details:", error);
      setSimulationDetailsError(getApiErrorMessage(error, "Could not load simulation details."));
    } finally {
      setSimulationDetailsLoading(false);
    }
  }

  /**
   * Archives the selected input artifact and refreshes the active project and library file lists.
   * Existing domain snapshots and simulation results are deliberately left unchanged.
   */
  async function handleConfirmDeleteInput() {
    const artifact = pendingArtifactDeletion;

    if (!artifact || !selectedProjectId || artifact.projectId !== selectedProjectId || deletingArtifactId !== null) {
      return;
    }

    const projectId = selectedProjectId;

    setDeletingArtifactId(artifact.id);
    setArtifactDeleteError(null);

    try {
      await deleteProjectInputArtifact(projectId, artifact.id);

      /*
       * Remove the archived artifact from the visible lists immediately after success.
       */
      setInputLibraryArtifacts(current => current.filter(item => item.id !== artifact.id));
      setArtifacts(current => current.filter(item => item.id !== artifact.id));

      setPendingArtifactDeletion(null);

      /*
       * Re-fetch authoritative data from the backend.
       * This also keeps the global library synchronized.
       */
      try {
        const nextArtifacts = await fetchProjectArtifacts(projectId);
        setArtifacts(nextArtifacts);

        if (selectedInputFormat) {
          const type = selectedInputFormat === "DSG" ? "DSG_INPUT" : "TPO_INPUT";
          const nextLibrary = await fetchArtifactLibrary(type);
          setInputLibraryArtifacts(nextLibrary);
        }
      } catch (refreshError) {
        console.error("Artifact archived, but list refresh failed:", refreshError);
        setError("The file was deleted, but the file list could not be refreshed.");
      }
    } catch (error) {
      console.error("Input artifact deletion failed:", error);
      setArtifactDeleteError(getApiErrorMessage(error, "Could not delete the selected file."));

    } finally {
      setDeletingArtifactId(null);
    }
  }

  // Reopens one persisted result selected from simulation history.
  async function handleViewSimulationResults(run: SimulationHistoryItem) {
    if (!selectedProjectId) {
      return;
    }

    // Avoid stacking two Escape-aware modals. The result window becomes the active view.
    setSimulationHistoryOpen(false);
    await openSimulationReviewSingle(selectedProjectId, run.id);
  }

  // Persists the project-wide display unit.
  // No hydraulic value is rewritten; all mounted project views react to the updated preference.
  async function handleApplyProjectDisplayUnit(unit: DisplayLengthUnit) {
    if (!selectedProjectId || unitSettingsSaving) {
      return;
    }
    setUnitSettingsSaving(true);
    setUnitSettingsError(null);
    try {
      const updated = await updateProjectDisplayUnit(selectedProjectId, unit);

      setProjects(
        current =>
          current.map(project => project.id === updated.id ? updated : project)
      );
      setUnitSettingsOpen(false);
    } catch (error) {
      console.error("Could not update project display unit:", error);
      setUnitSettingsError(getApiErrorMessage(error, "Could not apply unit settings."));
    } finally {
      setUnitSettingsSaving(false);
    }
  }

  function buildRawGraph(graph: NetworkGraph | null): NetworkGraph | null {
    if (!graph) return null;

    return {
      ...graph,
      id: `${graph.id}-raw`,
      name: `${graph.name} - Raw 2D coordinates`,
      nodes: graph.nodes.map((node) => ({
        ...node,
        x: node.sourceX ?? node.x,
        y: node.sourceY ?? node.y,
      })),
    };
  }

  function handleLocateNode(nodeId: string) {
    setNodeFocusRequest(current => ({ nodeId, requestId: (current?.requestId ?? 0) + 1 }));
  }

  function handleOpenNewProjectModal() {
    setOpenProjectModalOpen(false);
    setNewProjectModalOpen(true);
  }

  function handleOpenExistingProjectModal() {
    setNewProjectModalOpen(false);
    setOpenProjectModalOpen(true);

    void refreshProjects().catch(error => {
      console.error("Could not refresh project list:", error);
    });
  }

  function handleSelectProjectFromModal(projectId: string) {
    setError(null);
    setSelectedProjectId(projectId);
    setOpenProjectModalOpen(false);
  }

  function handleRequestDeleteProject(project: ProjectSummary) {
    setError(null);
    setProjectPendingDeletion(project);
  }

  function handleCancelProjectDeletion() {
    if (deletingProjectId !== null) {
      return;
    }
    setProjectPendingDeletion(null);
  }

  function handleOpenInputTypeModal() {
    if (!selectedProjectId) {
      return;
    }
    setInputLibraryError(null);
    setSelectedInputFormat(null);
    setUploadingInputFile(false);
    setInputTypeModalOpen(true);
  }

  function networkNameFromFilename(filename: string): string {
    return filename.replace(/\.(dsg|tpo)$/i, "").trim() || "Network";
  }

  /**
 * Detects the network format from a local filename.
 * Format detection is intentionally based only on supported extensions; file contents are validated later by the backend import pipeline.
 */
  function networkInputFormatFromFilename(filename: string): NetworkInputFormat | null {
    const lower = filename.trim().toLowerCase();

    if (lower.endsWith(".dsg")) {
      return "DSG";
    }
    if (lower.endsWith(".tpo")) {
      return "TPO";
    }

    return null;
  }

  // Enforces the project's DSG/TPO exclusivity before a local file is uploaded.
  function validateNetworkImportFormat(format: NetworkInputFormat): string | null {
    if (importBlocked) {
      return ("Network import is blocked because " + "this project has conflicting input formats.");
    }
    if (allowedImportFormat !== null && allowedImportFormat !== format) {
      return (
        `This project uses ${allowedImportFormat} input. `
        +
        `A ${format} file cannot be added to it.`
      );
    }


    return null;
  }

  function handleBackToInputType() {
    if (importingLibraryArtifactId) {
      return;
    }

    setInputLibraryModalOpen(false);

    setInputLibraryArtifacts([]);
    setInputLibraryError(null);

    setInputTypeModalOpen(true);
  }

  function handleCloseInputWorkflow() {
    if (importingLibraryArtifactId || tpoImporting) {
      return;
    }
    setInputTypeModalOpen(false);
    setInputLibraryModalOpen(false);
    setTpoDatabaseModalOpen(false);
    setSelectedInputFormat(null);
    setInputLibraryArtifacts([]);
    setInputLibraryError(null);
    setPendingTpoArtifact(null);
    setTpoRequirements(null);
    setTpoRequirementsLoading(false);
    setTpoConfigError(null);
    setDiametersLibraryArtifacts([]);
    setOrificesLibraryArtifacts([]);
    setUploadingInputFile(false);
  }

  function handleBackFromTpoConfiguration() {
    if (tpoImporting) {
      return;
    }
    setTpoDatabaseModalOpen(false);

    setPendingTpoArtifact(null);
    setTpoRequirements(null);

    setTpoConfigError(null);

    setDiametersLibraryArtifacts([]);
    setOrificesLibraryArtifacts([]);

    setInputLibraryModalOpen(true);
  }

  /**
   * Opens the confirmation dialog for an input artifact owned by the currently selected project.
   */
  function handleRequestDeleteInput(artifact: ProjectArtifact) {
    if (!selectedProjectId || artifact.projectId !== selectedProjectId || importingLibraryArtifactId !== null) {
      return;
    }

    setArtifactDeleteError(null);
    setPendingArtifactDeletion(artifact);
  }

  function handleCancelDeleteInput() {
    if (deletingArtifactId !== null) {
      return;
    }

    setPendingArtifactDeletion(null);
    setArtifactDeleteError(null);
  }

  function handleRequestDeleteSimulation(run: SimulationHistoryItem) {

    const terminal = run.status === "COMPLETED"
      ||
      run.status === "COMPLETED_WITH_ERRORS"
      ||
      run.status === "FAILED"
      ||
      run.status === "CANCELLED";

    if (!terminal) {
      setSimulationHistoryError("Cancel the simulation before deleting it.");
      return;
    }

    setSimulationDeleteError(null);
    setPendingSimulationDeletion(run);
  }

  function handleCancelSimulationDeletion() {
    if (deletingSimulationId !== null) {
      return;
    }
    setPendingSimulationDeletion(null);
    setSimulationDeleteError(null);
  }

  // Closes Hydraulic Review and then presents the next completed simulation waiting in the automatic queue.
  function handleCloseSimulationReview() {
    setSimulationReviewOpen(false);
    setSimulationReviewMode("SINGLE");
    setSimulationReviewRunId(null);
    setSimulationResultDetails(null);
    setSimulationResultError(null);
    setSimulationResultLoading(false);
    reviewDialogBusyRef.current = false;

    if (!selectedProjectId) {
      return;
    }
    window.setTimeout(
      () => {
        openNextQueuedSimulationResult(selectedProjectId);
      }, 0
    );
  }

  // Opens the Unit Settings dialog 
  function handleOpenUnitSettings() {
    if (!selectedProjectId) {
      setError("Open a project before changing unit settings.");
      return;
    }
    setUnitSettingsError(null);
    setUnitSettingsOpen(true);
  }

  // Returns the human-readable graph mode currently selected inside the shared Graph menu.
  function graphViewLabel(): string {
    switch (networkViewMode) {
      case "rawGraph":
        return "Raw 2D Graph";

      case "linearGraph":
        return "Linear Graph";

      case "visualGraph":
      default:
        return "Visual Graph";
    }
  }

  // Open the simulation review dialog with the latest completed simulation.
  function handleOpenSimulationReview() {
    if (!selectedProjectId || simulationRuns.length === 0) {
      return;
    }
    const latestRun = simulationRuns[simulationRuns.length - 1];
    reviewDialogBusyRef.current = true;
    setSimulationReviewOpen(true);
    setSimulationReviewMode("SINGLE");
    void loadSimulationReviewRun(selectedProjectId, latestRun.id);
  }

  // Selects a completed simulation to review, either in single mode or as part of a comparison.
  function handleSelectSimulationReviewRun(runId: string) {
    if (!selectedProjectId) {
      return;
    }

    void loadSimulationReviewRun(selectedProjectId, runId);
  }

  // Changes the mode of the simulation review dialog (single or comparison).
  function handleSimulationReviewModeChange(mode: SimulationReviewMode) {
    if (mode === "COMPARISON" && reviewComparisonSources.length < 2) {
      return;
    }
    setSimulationReviewMode(mode);
  }

  // Drop old client-demand fields that no longer belong in SimulationRunRequestOptions
  function stripLegacyDemandOverrides(options: Record<string, unknown>): SimulationRunRequestOptions {
    const {
      openTapsProbability: _legacyProbability,
      targetFlowLps: _legacyTargetFlow,
      ...remaining
    } = options;
    return remaining as SimulationRunRequestOptions;
  }

  useEffect(() => {
    setNodeFocusRequest(null);
  }, [graph?.id]);

  // Closes the graph view menu when the user clicks or presses a key outside the menu.
  useEffect(() => {
    if (!graphViewMenuOpen) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      const target = event.target as Node;
      if (graphViewMenuRef.current && !graphViewMenuRef.current.contains(target)) {
        setGraphViewMenuOpen(false);
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setGraphViewMenuOpen(false);
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };

  }, [graphViewMenuOpen]);

  function handleStartApplication() {
    setAppPhase("transition");
  }

  function handleTransitionComplete() {
    setAppPhase("workspace");
  }

  if (appPhase === "start") {
    return (
      <StartScreen onStart={handleStartApplication} />
    );
  }

  if (appPhase === "transition") {
    return <WaterIntroTransition onComplete={handleTransitionComplete} />;
  }

  return (
    <main className="app-shell">

      <AppMenuBar
        activeProjectName={selectedProject?.name ?? null}
        canImportNetwork={selectedProjectId !== null}
        onNewProject={handleOpenNewProjectModal}
        onOpenProject={handleOpenExistingProjectModal}
        onImportNetwork={handleOpenInputTypeModal}
        onOpenDatabase={handleOpenDatabase}
        canConfigureUnits={selectedProject !== null}
        onOpenUnitSettings={handleOpenUnitSettings}
      />

      {error && <div className="error-banner">{error}</div>}

      {hasImportedDomain && (
        <SimulationPanel
          historyCount={simulationHistory.length}
          onOpenHistory={handleOpenSimulationHistory}
          disabled={!selectedProjectId}
          activeRunCount={activeSimulationCount}
          foregroundRunning={foregroundSimulationJobId !== null}
          cancelling={simulationCancelling}

          defaults={simulationDefaults}
          summary={simulationSummary}
          statistics={simulationStatistics}
          canRunAgain={lastSimulationOptions !== null}
          canReview={Boolean(latestDomainSnapshot) && comparisonRuns.length > 0}
          onOpenReview={handleOpenSimulationReview}

          exportControl={
            <ExportControl
              disabled={!selectedProjectId || simulationRunning}
              exporting={simulationExporting}
              availability={exportAvailability}
              onExport={handleProjectExport}
            />
          }
          onRunDefault={handleRunDefaultSimulation}
          onRun={handleRunSimulation}
          onRunAgain={handleRunSimulationAgain}
          onCancel={handleCancelSimulation}
        />
      )}

      <section className="network-first-layout">

        <section className="network-stage">
          {!hasImportedDomain && (
            <WorkspaceEmptyState
              projectName={selectedProject?.name ?? null}
              onNewProject={handleOpenNewProjectModal}
              onOpenProject={handleOpenExistingProjectModal}
              onImportNetwork={handleOpenInputTypeModal}
            />
          )}

          {hasImportedDomain && (
            <>
              <div className="network-view-toolbar">
                <div className="network-view-tabs">
                  <div ref={graphViewMenuRef} className="network-graph-menu">
                    <button
                      type="button"
                      className={graphViewActive
                        ? "network-view-tab active network-graph-menu__trigger"
                        : "network-view-tab network-graph-menu__trigger"
                      }
                      aria-haspopup="menu"
                      aria-expanded={graphViewMenuOpen}
                      onClick={() => {
                        setGraphViewMenuOpen(current => !current);
                      }}
                    >
                      <span>Graph</span>
                      <span className="network-graph-menu__chevron" aria-hidden="true">▾</span>
                    </button>

                    {graphViewMenuOpen && (
                      <div className="network-graph-menu__dropdown" role="menu">
                        <button
                          type="button"
                          role="menuitemradio"
                          aria-checked={networkViewMode === "visualGraph"}
                          className={networkViewMode === "visualGraph"
                            ? "network-graph-menu__item selected"
                            : "network-graph-menu__item"
                          }
                          onClick={() => {
                            setNetworkViewMode("visualGraph");
                            setGraphViewMenuOpen(false);
                          }}
                        >
                          <span className="network-graph-menu__check" aria-hidden="true">
                            {networkViewMode === "visualGraph" ? "✓" : ""}
                          </span>
                          <span>Visual Graph</span>
                        </button>

                        <button
                          type="button"
                          role="menuitemradio"
                          aria-checked={networkViewMode === "rawGraph"}
                          className={networkViewMode === "rawGraph"
                            ? "network-graph-menu__item selected"
                            : "network-graph-menu__item"
                          }
                          onClick={() => {
                            setNetworkViewMode("rawGraph");
                            setGraphViewMenuOpen(false);
                          }}
                        >
                          <span className="network-graph-menu__check" aria-hidden="true">
                            {networkViewMode === "rawGraph" ? "✓" : ""}
                          </span>
                          <span>Raw 2D Graph</span>
                        </button>
                        <button
                          type="button"
                          role="menuitemradio"
                          aria-checked={networkViewMode === "linearGraph"}
                          className={networkViewMode === "linearGraph"
                            ? "network-graph-menu__item selected"
                            : "network-graph-menu__item"
                          }
                          onClick={() => {
                            setNetworkViewMode("linearGraph");
                            setGraphViewMenuOpen(false);
                          }}
                        >
                          <span className="network-graph-menu__check" aria-hidden="true">
                            {networkViewMode === "linearGraph" ? "✓" : ""}
                          </span>
                          <span>Linear Graph</span>
                        </button>

                      </div>
                    )}

                  </div>

                  <button
                    type="button"
                    className={networkViewMode === "tables" ? "network-view-tab active" : "network-view-tab"}
                    aria-pressed={networkViewMode === "tables"}
                    onClick={() => {
                      setGraphViewMenuOpen(false);
                      setNetworkViewMode("tables");
                    }}
                  >
                    Tables
                  </button>

                  <button
                    type="button"
                    className={networkViewMode === "text" ? "network-view-tab active" : "network-view-tab"}
                    onClick={() => {
                      setGraphViewMenuOpen(false);
                      setNetworkViewMode("text");
                    }}
                  >
                    Text
                  </button>

                </div>

                {graphViewActive &&
                  searchableGraph !== null && (
                    <NetworkNodeSearch nodes={searchableGraph.nodes} onLocate={handleLocateNode} />
                  )}
              </div>

              <div className="network-view-body">

                {networkViewMode === "visualGraph" &&
                  graph !== null && (
                    <NetworkCanvas
                      graph={graph}
                      selection={selection}
                      focusRequest={nodeFocusRequest}
                      onSelectionChange={setSelection}
                      onToggleEdgeStatus={handleToggleEdgeStatus}
                      onMoveNode={handleMoveNode}
                      coordinateMode="visual"
                      displayUnit={projectDisplayUnit}
                    />
                  )}

                {networkViewMode === "rawGraph" &&
                  rawGraph !== null && (
                    <NetworkCanvas
                      graph={rawGraph}
                      selection={selection}
                      focusRequest={nodeFocusRequest}
                      onSelectionChange={setSelection}
                      onToggleEdgeStatus={handleToggleEdgeStatus}
                      onMoveNode={handleMoveNode}
                      coordinateMode="raw"
                      displayUnit={projectDisplayUnit}
                    />
                  )}


                {networkViewMode === "linearGraph" &&
                  graph !== null && (
                    <LinearNetworkView
                      graph={graph}
                      selection={selection}
                      focusRequest={nodeFocusRequest}
                      onSelectionChange={setSelection}
                      onToggleEdgeStatus={handleToggleEdgeStatus}
                      onMoveNode={handleMoveNode}
                      displayUnit={projectDisplayUnit}
                    />
                  )}

                {networkViewMode === "tables" &&
                  latestDomainSnapshot !== null && (
                    <DomainDataViewer
                      snapshot={latestDomainSnapshot}
                      displayUnit={projectDisplayUnit}
                      sourceArtifact={activeSourceArtifact}
                      onSaveRevision={handleSaveDomainRevision}
                    />
                  )}

                {networkViewMode === "text" && latestDomainSnapshot !== null && (
                  <DomainTextViewer
                    projectId={latestDomainSnapshot.projectId}
                    sourceArtifact={activeSourceArtifact}
                    domain={latestDomainSnapshot.domain}
                  />
                )}

              </div>
            </>
          )}

        </section>

      </section>

      {newProjectModalOpen && (
        <NewProjectModal
          onCreate={handleCreateProjectFromModal}
          onClose={() => setNewProjectModalOpen(false)}
        />
      )}

      {openProjectModalOpen && (
        <OpenProjectModal
          projects={projects}
          loading={projectsLoading}
          loadError={projectLoadError}
          selectedProjectId={selectedProjectId}
          onSelect={handleSelectProjectFromModal}
          onRetry={() => { void refreshProjects().catch(() => { }); }}
          onRequestDelete={handleRequestDeleteProject}
          onClose={() => setOpenProjectModalOpen(false)}
        />
      )}

      {projectPendingDeletion && (
        <DeleteProjectConfirmModal
          project={projectPendingDeletion}
          deleting={deletingProjectId === projectPendingDeletion.id}
          onCancel={handleCancelProjectDeletion}
          onConfirm={handleConfirmDeleteProject}
        />
      )}
      {inputTypeModalOpen &&
        selectedProject && (
          <InputTypeModal
            projectName={selectedProject.name}
            allowedFormat={allowedImportFormat}
            blocked={importBlocked}
            uploading={uploadingInputFile}
            error={inputLibraryError}
            onDropFile={handleDropNetworkInput}
            onSelect={handleSelectInputFormat}
            onClose={handleCloseInputWorkflow}
          />
        )}

      {inputLibraryModalOpen &&
        selectedProjectId &&
        selectedInputFormat && (
          <InputFileLibraryModal
            format={selectedInputFormat}
            artifacts={inputLibraryArtifacts}
            projects={projects}
            selectedProjectId={selectedProjectId}
            loading={inputLibraryLoading}
            importingArtifactId={importingLibraryArtifactId}
            error={inputLibraryError}
            onImport={handleImportLibraryArtifact}
            onBack={handleBackToInputType}
            onClose={handleCloseInputWorkflow}
            uploading={uploadingInputFile}
            onUpload={file => {
              if (!selectedInputFormat) {
                return Promise.resolve();
              }
              return handleUploadNetworkInput(file, selectedInputFormat, false);
            }}
            onRequestDelete={handleRequestDeleteInput}
          />
        )}

      {pendingArtifactDeletion && (
        <DeleteInputArtifactModal
          artifact={pendingArtifactDeletion}
          deleting={deletingArtifactId === pendingArtifactDeletion.id}
          error={artifactDeleteError}
          onConfirm={handleConfirmDeleteInput}
          onCancel={handleCancelDeleteInput}
        />
      )}

      {tpoDatabaseModalOpen &&
        pendingTpoArtifact && (
          <TpoDatabaseModal
            tpoArtifact={pendingTpoArtifact}
            requirements={tpoRequirements}
            loadingRequirements={tpoRequirementsLoading}
            diametersArtifacts={diametersLibraryArtifacts}
            orificesArtifacts={orificesLibraryArtifacts}
            projects={projects}
            importing={tpoImporting}
            error={tpoConfigError}
            onUploadDatabase={handleUploadTpoDatabase}
            onImport={handleConfirmTpoImport}
            onBack={handleBackFromTpoConfiguration}
            onClose={handleCloseInputWorkflow}
          />
        )}

      {activeDatabaseType && (
        <DatabaseManagerModal
          type={activeDatabaseType}
          artifacts={databaseArtifacts}
          projects={projects}
          loading={databaseLoading}
          uploading={databaseUploading}
          error={databaseError}
          onUpload={handleUploadDatabaseFile}
          canUpload={selectedProjectId !== null}
          onArchive={handleArchiveDatabase}
          onClose={() => setActiveDatabaseType(null)}
        />
      )}

      {simulationHistoryOpen && (
        <SimulationHistoryDialog
          runs={simulationHistory}
          loading={simulationHistoryLoading}
          cancellingId={historyCancellingRunId}
          error={simulationHistoryError}
          onViewDetails={handleViewSimulationDetails}
          onViewResults={handleViewSimulationResults}
          onCancelRun={handleCancelHistorySimulation}
          onRequestDelete={handleRequestDeleteSimulation}
          onClose={() => setSimulationHistoryOpen(false)}
        />
      )}
      {pendingSimulationDeletion && (
        <DeleteSimulationDialog
          run={pendingSimulationDeletion}
          deleting={deletingSimulationId === pendingSimulationDeletion.id}
          error={simulationDeleteError}
          onConfirm={handleConfirmDeleteSimulation}
          onCancel={handleCancelSimulationDeletion}
        />
      )}

      {simulationDetailsOpen && (
        <SimulationDetailsDialog
          details={simulationDetails}
          loading={simulationDetailsLoading}
          error={simulationDetailsError}
          onClose={() => {
            setSimulationDetailsOpen(false);
            setSimulationDetails(null);
            setSimulationDetailsError(null);
          }}
          displayUnit={projectDisplayUnit}
        />
      )}

      {simulationReviewOpen && selectedProjectId && (
        <SimulationReviewDialog
          projectId={selectedProjectId}
          mode={simulationReviewMode}
          onModeChange={handleSimulationReviewModeChange}
          runs={simulationRuns}
          selectedRunId={simulationReviewRunId}
          onSelectRun={handleSelectSimulationReviewRun}
          details={simulationResultDetails}
          loading={simulationResultLoading}
          error={simulationResultError}
          comparisonSources={reviewComparisonSources}
          loadComparisonDataset={loadComparisonDataset}
          comparisonWorkspace={comparisonWorkspace}
          onComparisonWorkspaceChange={handleComparisonWorkspaceChange}
          displayUnit={projectDisplayUnit}
          onClose={handleCloseSimulationReview}
        />

      )}

      {unitSettingsOpen && selectedProject && (
        <UnitSettingsModal
          projectName={selectedProject.name}
          currentUnit={projectDisplayUnit}
          saving={unitSettingsSaving}
          error={unitSettingsError}
          onApply={handleApplyProjectDisplayUnit}
          onClose={() => {
            if (unitSettingsSaving) {
              return;
            }
            setUnitSettingsOpen(false);
            setUnitSettingsError(null);
          }}
        />
      )}
    </main>
  );
}