package com.neatwork.backend.controller;

import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.request.*;
import com.neatwork.backend.response.DomainImportResponse;
import com.neatwork.backend.response.ProjectDomainSnapshotResponse;
import com.neatwork.backend.response.*;
import com.neatwork.backend.service.*;
import jakarta.validation.Valid;
import lombok.RequiredArgsConstructor;
import org.springframework.core.io.Resource;
import org.springframework.http.HttpHeaders;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.util.List;
import java.util.UUID;

@RestController
@RequestMapping("/api/projects")
@RequiredArgsConstructor
public class ProjectController {

    private final ProjectService projectService;
    private final NetworkMutationService networkMutationService;
    private final ProjectArtifactService projectArtifactService;
    private final ProjectDomainImportService projectDomainImportService;
    private final SimulationJobService simulationJobService;
    private final ProjectExportService projectExportService;
    private final SimulationRunService simulationRunService;
    private final ProjectDomainRevisionService projectDomainRevisionService;
    private final DomainSnapshotCompatibilityService domainSnapshotCompatibilityService;

    @GetMapping
    public List<ProjectSummaryResponse> listProjects() {
        return projectService.listProjects();
    }

    @PostMapping
    public ProjectSummaryResponse createProject(@Valid @RequestBody CreateProjectRequest request) {
        return projectService.createProject(request);
    }

    /**
     * Delete one complete project.
     *
     * @param projectId project identifier
     * @return HTTP 204 when deletion succeeds
     */
    @DeleteMapping("/{projectId}")
    public ResponseEntity<Void> deleteProject(@PathVariable UUID projectId) {
        projectService.deleteProject(projectId);

        return ResponseEntity.noContent().build();
    }

    @GetMapping("/{projectId}/graph")
    public NetworkGraphResponse getProjectGraph(@PathVariable UUID projectId) {
        return projectService.getProjectGraph(projectId);
    }


    @PatchMapping("/{projectId}/edges/{edgeId}/status")
    public NetworkEdgeResponse updateEdgeStatus(
            @PathVariable UUID projectId,
            @PathVariable String edgeId,
            @Valid @RequestBody UpdateEdgeStatusRequest request
            ) {
        return networkMutationService.updateEdgeStatus(projectId, edgeId, request.status());
    }

    @PatchMapping("/{projectId}/nodes/{nodeId}/position")
    public NetworkNodeResponse updateNodePosition(
            @PathVariable UUID projectId,
            @PathVariable String nodeId,
            @Valid @RequestBody UpdateNodePositionRequest request
            ) {
        return networkMutationService.updateNodePosition(projectId, nodeId, request.x(), request.y());
    }

    @GetMapping("/{projectId}/artifacts")
    public List<ProjectArtifactResponse> listArtifacts(@PathVariable UUID projectId) {
        return projectArtifactService.listArtifacts(projectId);
    }

    @PostMapping(value = "/{projectId}/artifacts", consumes = MediaType.MULTIPART_FORM_DATA_VALUE)
    public ProjectArtifactResponse uploadArtifact(
            @PathVariable UUID projectId,
            @RequestParam ProjectArtifactType type,
            @RequestPart MultipartFile file
            ) {
        return projectArtifactService.storeInputArtifact(projectId, type, file);
    }

    @PostMapping("/{projectId}/artifacts/{sourceArtifactId}/copy")
    public ProjectArtifactResponse copyArtifactToProject(
            @PathVariable UUID projectId,
            @PathVariable UUID sourceArtifactId
    ) {
        return projectArtifactService.copyInputArtifactToProject(projectId, sourceArtifactId);
    }

    // Returns the original textual content of one active project artifact without forcing a file download.
    // DSG and TPO source files are exposed exactly as they are stored in the project workspace.
    @GetMapping(value = "/{projectId}/artifacts/{artifactId}/text", produces = MediaType.TEXT_PLAIN_VALUE)
    public ResponseEntity<String> readArtifactText(@PathVariable UUID projectId, @PathVariable UUID artifactId) {
        var content = projectArtifactService.readArtifactText(projectId, artifactId);

        return ResponseEntity.ok().contentType(new MediaType(
                "text", "plain", java.nio.charset.StandardCharsets.UTF_8))
                .body(content);
    }

    @GetMapping("/{projectId}/artifacts/{artifactId}/download")
    public ResponseEntity<Resource> downloadArtifact(
            @PathVariable UUID projectId,
            @PathVariable UUID artifactId
    ) {
        var resource = projectArtifactService.loadArtifact(projectId, artifactId);

        return ResponseEntity.ok()
                .header(
                        HttpHeaders.CONTENT_DISPOSITION,
                        "attachment; filename=\"" + resource.getFilename() + "\""
                )
                .contentType(MediaType.APPLICATION_OCTET_STREAM)
                .body(resource);
    }

    @PostMapping("/{projectId}/domain/import-from-artifacts")
    public DomainImportResponse importDomainFromArtifacts(
            @PathVariable UUID projectId,
            @Valid @RequestBody ImportFromArtifactsRequest request
    ) {
        return projectDomainImportService.importFromArtifacts(projectId, request);
    }

    @GetMapping("/{projectId}/domain/latest")
    public ProjectDomainSnapshotResponse getLatestDomainSnapshot(@PathVariable UUID projectId) {
        return projectDomainImportService.getLatestSnapshot(projectId);
    }

    @PostMapping("/{projectId}/simulation/jobs")
    public ResponseEntity<SimulationJobResponse> startSimulationJob(@PathVariable UUID projectId,
            @Valid
            @RequestBody(required = false)
            SimulationOptionsRequest options
    ) {
        var job = simulationJobService.start(projectId, options);

        return ResponseEntity.accepted().body(job);
    }

    // Return persisted unfinished simulation jobs.
    @GetMapping("/{projectId}/simulation/jobs")
    public List<SimulationJobResponse> listActiveSimulationJobs(@PathVariable UUID projectId) {
        return simulationJobService.listActive(projectId);
    }

    @GetMapping("/{projectId}/simulation/jobs/{jobId}")
    public SimulationJobResponse getSimulationJob(@PathVariable UUID projectId, @PathVariable UUID jobId) {
        return simulationJobService.get(projectId, jobId);
    }

    @DeleteMapping("/{projectId}/simulation/jobs/{jobId}")
    public SimulationJobResponse cancelSimulationJob(@PathVariable UUID projectId, @PathVariable UUID jobId) {
        return simulationJobService.cancel(projectId, jobId);
    }

    @GetMapping("/{projectId}/exports/availability")
    public List<ProjectExportAvailabilityResponse> getExportAvailability(@PathVariable UUID projectId) {

        return projectExportService.getAvailability(projectId);
    }

    @PostMapping("/{projectId}/exports")
    public ProjectArtifactResponse exportProject(@PathVariable UUID projectId, @Valid @RequestBody ProjectExportRequest request) {
        return projectExportService.export(projectId, request.format());
    }

    /**
     * Return the complete persisted simulation history of one project.
     */
    @GetMapping("/{projectId}/simulation/runs")
    public List<SimulationRunSummaryResponse> listSimulationRuns(@PathVariable UUID projectId) {
        return simulationRunService.listRuns(projectId);
    }

    /**
     * Return the complete statistics and options of one historical run.
     */
    @GetMapping("/{projectId}/simulation/runs/{runId}")
    public SimulationRunDetailsResponse getSimulationRun(@PathVariable UUID projectId, @PathVariable UUID runId) {
        return simulationRunService.getRun(projectId, runId);
    }

    /**
     * Removes one DSG or TPO input file from the project's active artifact library.
     * The physical file and historical simulation dependencies are preserved.
     * @return HTTP 204 when the operation succeeds
     */
    @DeleteMapping("/{projectId}/artifacts/{artifactId}")
    public ResponseEntity<Void> deleteInputArtifact(@PathVariable UUID projectId, @PathVariable UUID artifactId) {
        projectArtifactService.archiveNetworkInput(projectId, artifactId);

        return ResponseEntity.noContent().build();
    }

    // Saves edited input data as a new immutable network-file revision.
    @PostMapping("/{projectId}/domain/revisions")
    public ResponseEntity<DomainRevisionResponse> saveDomainRevision(@PathVariable UUID projectId, @Valid @RequestBody SaveDomainRevisionRequest request) {
        var response = projectDomainRevisionService
                        .saveRevision(projectId, request);

        return ResponseEntity.status(HttpStatus.CREATED).body(response);
    }

    // Return every simulation attempt for this project.
    @GetMapping("/{projectId}/simulation/history")
    public List<SimulationHistoryItemResponse> listSimulationHistory(@PathVariable UUID projectId) {
        return simulationRunService.listHistory(projectId);
    }

    /**
     * Delete one terminal simulation execution.
     * Running executions must be cancelled first.
     */
    @DeleteMapping("/{projectId}/simulation/runs/{runId}")
    public ResponseEntity<Void> deleteSimulationRun(@PathVariable UUID projectId, @PathVariable UUID runId) {
        simulationRunService.deleteRun(projectId, runId);
        return ResponseEntity.noContent().build();
    }

    // Updates the project-wide display unit.
    @PatchMapping("/{projectId}/settings/display-unit")
    public ProjectSummaryResponse
    updateProjectDisplayUnit(@PathVariable UUID projectId, @Valid @RequestBody UpdateProjectDisplayUnitRequest request) {
        return projectService.updateDisplayUnit(projectId, request.displayUnit());
    }

    @GetMapping(
            "/{projectId}/domain/comparison/compatibility"
    )
    public DomainSnapshotCompatibilityResponse
    compareDomainSnapshots(@PathVariable UUID projectId, @RequestParam UUID leftSnapshotId, @RequestParam UUID rightSnapshotId) {
        return domainSnapshotCompatibilityService.compare(projectId, leftSnapshotId, rightSnapshotId);
    }
}
