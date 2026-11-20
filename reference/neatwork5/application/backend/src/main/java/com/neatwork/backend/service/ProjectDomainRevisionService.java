package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.dao.ProjectRepository;

import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.enums.RevisionNamingMode;

import com.neatwork.backend.exception.ResourceNotFoundException;

import com.neatwork.backend.model.entity.ProjectDomainSnapshotEntity;

import com.neatwork.backend.model.mapper.ProjectArtifactMapper;

import com.neatwork.backend.request.NeatworkRevisionSerializeRequest;
import com.neatwork.backend.request.NetworkGraphSaveRequest;
import com.neatwork.backend.request.SaveDomainRevisionRequest;

import com.neatwork.backend.response.DomainRevisionResponse;
import com.neatwork.backend.response.ProjectDomainSnapshotResponse;

import lombok.RequiredArgsConstructor;

import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.UUID;


@Service
@RequiredArgsConstructor
public class ProjectDomainRevisionService {

    private final ProjectRepository projectRepository;
    private final ProjectDomainSnapshotRepository snapshotRepository;
    private final ProjectArtifactRepository artifactRepository;
    private final ProjectArtifactService artifactService;
    private final ProjectArtifactMapper artifactMapper;
    private final PythonNeatworkRevisionClient pythonRevisionClient;
    private final NetworkGraphSaveService graphSaveService;
    private final DomainCatalogIntegrityService domainCatalogIntegrityService;


    /**
     * Saves edited input data as a completely new DSG/TPO revision.
     * The original artifact and original snapshot remain immutable.
     */
    @Transactional
    public DomainRevisionResponse saveRevision(UUID projectId, SaveDomainRevisionRequest request) {
        var project = projectRepository
                .findByIdForUpdate(projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Project not found: " + projectId));

        var sourceSnapshot = snapshotRepository
                .findByIdAndProjectId(request.sourceSnapshotId(), projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Source snapshot not found."));

        if (sourceSnapshot.getPrimaryArtifactId() == null) {
            throw new IllegalArgumentException("This legacy snapshot has no source artifact. " + "Re-import the network before editing it.");
        }

        var sourceArtifact = artifactRepository
                .findByIdAndProjectId(sourceSnapshot.getPrimaryArtifactId(), projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Source input file not found."));

        if (sourceArtifact.isArchived()) {
            throw new IllegalArgumentException("The source file has been archived.");
        }

        var format = switch (sourceArtifact.getType()) {
            case DSG_INPUT -> "DSG";
            case TPO_INPUT -> "TPO";
            default -> throw new IllegalArgumentException("Only DSG/TPO input files can be edited.");
        };

        /*
         * The project format cannot change while creating a revision.
         */
        if (!"NONE".equalsIgnoreCase(project.getSourceFormat().name())
                &&
                !project.getSourceFormat().name().equalsIgnoreCase(format)) {
            throw new IllegalArgumentException("Revision format does not match project format.");
        }

        var sourceContent = artifactService.readArtifactText(projectId, sourceArtifact.getId());

        var domainName = String.valueOf(request.domain().getOrDefault("name", "Network"));
        domainCatalogIntegrityService.validate(request.domain());

        var serialized = pythonRevisionClient.serialize(
                new NeatworkRevisionSerializeRequest(domainName, format, sourceContent, request.domain()));

        String customFilename = null;
        if (request.namingMode() == RevisionNamingMode.CUSTOM) {
            customFilename = request.customFilename();
        }

        var revision = artifactService.registerNetworkRevision(
                        projectId,
                        sourceArtifact,
                        customFilename,
                        serialized.content().getBytes(StandardCharsets.UTF_8)
                );

        /*
         * Use the new visible filename as the canonical domain name in future views.
         */
        var canonicalDomain = new HashMap<String, Object>(serialized.domain());
        canonicalDomain.put("name", removeExtension(revision.getOriginalFilename()));
        var snapshot = snapshotRepository.save(
                ProjectDomainSnapshotEntity
                        .builder()
                        .projectId(projectId)
                        .sourceFormat(format)
                        .primaryArtifactId(revision.getId())
                        /*
                         * Revised TPO files are self-contained, therefore external DB dependencies are no longer required by this snapshot.
                         */
                        .diametersArtifactId(null)
                        .orificesArtifactId(null)
                        .domainJson(canonicalDomain)
                        .build()
        );


        var savedGraph = graphSaveService.replaceGraph(projectId,
                new NetworkGraphSaveRequest(
                        serialized.graph().nodes(),
                        serialized.graph().edges())
        );
        return new DomainRevisionResponse(artifactMapper.toResponse(revision),
                toSnapshotResponse(snapshot), savedGraph
        );
    }

    private String removeExtension(String filename) {
        var dot = filename.lastIndexOf('.');
        return dot > 0 ? filename.substring(0, dot) : filename;
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
}