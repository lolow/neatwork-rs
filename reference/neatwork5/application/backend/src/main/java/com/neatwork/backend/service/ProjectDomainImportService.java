package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.enums.ProjectSourceFormat;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.entity.ProjectDomainSnapshotEntity;
import com.neatwork.backend.request.ImportFromArtifactsRequest;
import com.neatwork.backend.request.NeatworkParseRequest;
import com.neatwork.backend.request.NetworkGraphSaveRequest;
import com.neatwork.backend.response.DomainImportResponse;
import com.neatwork.backend.response.ProjectDomainSnapshotResponse;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.Locale;
import java.util.UUID;

@Slf4j
@Service
@RequiredArgsConstructor
public class ProjectDomainImportService {

    private final ProjectRepository projectRepository;
    private final ProjectArtifactService artifactService;
    private final PythonNeatworkImportClient pythonClient;
    private final ProjectDomainSnapshotRepository snapshotRepository;
    private final NetworkGraphSaveService graphSaveService;
    private final ProjectArtifactRepository artifactRepository;
    private final ProjectSourcePolicy sourcePolicy;

    @Transactional
    public DomainImportResponse importFromArtifacts(UUID projectId, ImportFromArtifactsRequest request) {
        ensureProjectExists(projectId);

        /*
         * Resolve and validate the requested input format.
         * Never trust the format supplied by the frontend without checking the actual artifact metadata.
         */
        ProjectSourceFormat requestedFormat;
        try {
            requestedFormat = ProjectSourceFormat.valueOf(request.format().trim().toUpperCase(Locale.ROOT));
        } catch (IllegalArgumentException exception) {
            throw new IllegalArgumentException("Unsupported network input format.", exception);
        }

        if (requestedFormat != ProjectSourceFormat.DSG && requestedFormat != ProjectSourceFormat.TPO) {
            throw new IllegalArgumentException("Only DSG and TPO can be imported " + "as primary network sources.");
        }

        var primaryArtifact = artifactRepository
                .findById(request.primaryArtifactId())
                .orElseThrow(() -> new ResourceNotFoundException("Primary artifact not found."));

        if (!primaryArtifact.getProjectId().equals(projectId)) {
            throw new IllegalArgumentException("The primary artifact does not belong " + "to the selected project.");
        }

        var expectedType = requestedFormat == ProjectSourceFormat.DSG
                        ? ProjectArtifactType.DSG_INPUT
                        : ProjectArtifactType.TPO_INPUT;

        if (primaryArtifact.getType() != expectedType || primaryArtifact.isGenerated()) {
            throw new IllegalArgumentException("The selected artifact does not match " + "the requested input format.");
        }

        /*
         * Lock the project and verify that the requested source format is compatible with its existing data.
         */
        var project = sourcePolicy.lockForImport(projectId, requestedFormat);

        var primaryContent = artifactService.readArtifactText(projectId, request.primaryArtifactId());
        if (primaryContent == null || primaryContent.isBlank()){
            throw new IllegalArgumentException("The selected primary artifact is empty");
        }
        if ("TPO".equalsIgnoreCase(request.format())
                && !hasSection(primaryContent, "Diameters")
                && request.diametersDbArtifactId() == null) {
            throw new IllegalArgumentException(
                    "TPO import requires a diameters database. The selected TPO file has no !Diameters section. "
                            + "Upload diameters.db as DB_DIAMETERS_INPUT and select it in Domain import."
            );
        }
        var diametersContent = request.diametersDbArtifactId() == null
                ? null
                : artifactService.readArtifactText(projectId, request.diametersDbArtifactId());
        var orificesContent = request.orificesDbArtifactId() == null
                ? null
                : artifactService.readArtifactText(projectId, request.orificesDbArtifactId());

        log.info(
                "Importing domain: projectId={}, format={}, primaryArtifactId={}, contentLength={}",
                projectId, request.format(), request.primaryArtifactId(), primaryContent.length()
        );

        var parsed = pythonClient.parse(new NeatworkParseRequest(
                request.name(),
                request.format(),
                primaryContent,
                diametersContent,
                orificesContent
        ));

        /*
         * Store exact input artifact identifiers.
         * Historical simulations must remain traceable to the files used when their domain was imported.
         */
        var snapshot = snapshotRepository.save(ProjectDomainSnapshotEntity.builder()
                .projectId(projectId)
                .sourceFormat(requestedFormat.name())
                .primaryArtifactId(request.primaryArtifactId())
                .diametersArtifactId(request.diametersDbArtifactId())
                .orificesArtifactId(request.orificesDbArtifactId())
                .domainJson(parsed.domain())
                .build());

        var savedGraph = graphSaveService.replaceGraph(projectId,
                new NetworkGraphSaveRequest(
                        parsed.graph().nodes(),
                        parsed.graph().edges()
                )
        );

        /*
         * Commit the project's active format only after the domain has been parsed and persisted.
         * A failed import must leave the project format unchanged.
         */
        if (project.getSourceFormat() == ProjectSourceFormat.NONE) {
            project.setSourceFormat(requestedFormat);
        }

        return new DomainImportResponse(
                toSnapshotResponse(snapshot),
                savedGraph
        );
    }

    @Transactional(readOnly = true)
    public ProjectDomainSnapshotResponse getLatestSnapshot(UUID projectId) {
        ensureProjectExists(projectId);

        var snapshot = snapshotRepository.findFirstByProjectIdOrderByCreatedAtDesc(projectId)
                .orElseThrow(() -> new ResourceNotFoundException("No domain snapshot found for project: " + projectId));

        return toSnapshotResponse(snapshot);
    }

    private void ensureProjectExists(UUID projectId) {
        if (!projectRepository.existsById(projectId)) {
            throw new ResourceNotFoundException("Project not found: " + projectId);
        }
    }

    private ProjectDomainSnapshotResponse toSnapshotResponse(ProjectDomainSnapshotEntity snapshot) {
        return new ProjectDomainSnapshotResponse(
                snapshot.getId(),
                snapshot.getProjectId(),
                snapshot.getSourceFormat(),
                snapshot.getDomainJson(),
                snapshot.getCreatedAt(),
                snapshot.getPrimaryArtifactId(),
                snapshot.getDiametersArtifactId(),
                snapshot.getOrificesArtifactId()
        );
    }

    private boolean hasSection(String content, String sectionName) {
        var expected = "!" + sectionName.toLowerCase();

        return content.lines()
                .map(String::trim)
                .map(String::toLowerCase)
                .anyMatch(line -> line.equals(expected));
    }
}