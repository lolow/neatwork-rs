package com.neatwork.backend.service;

import com.neatwork.backend.config.WorkspaceProperties;
import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.entity.ProjectArtifactEntity;
import com.neatwork.backend.model.mapper.ProjectArtifactMapper;
import com.neatwork.backend.response.ProjectArtifactResponse;
import com.neatwork.backend.response.TpoArtifactRequirementsResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.core.io.FileSystemResource;
import org.springframework.core.io.Resource;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.web.multipart.MultipartFile;

import java.io.IOException;
import java.nio.charset.MalformedInputException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.text.Normalizer;
import java.util.Comparator;
import java.util.List;
import java.util.UUID;

@Service
@RequiredArgsConstructor
public class ProjectArtifactService {

    private final WorkspaceProperties workspaceProperties;
    private final ProjectRepository projectRepository;
    private final ProjectArtifactRepository artifactRepository;
    private final ProjectArtifactMapper mapper;
    private final ProjectSourcePolicy sourcePolicy;

    @Transactional(readOnly = true)
    public List<ProjectArtifactResponse> listArtifacts(UUID projectId) {
        ensureProjectExists(projectId);

        return artifactRepository.findByProjectIdAndArchivedFalseOrderByCreatedAtDesc(projectId)
                .stream()
                .map(mapper::toResponse)
                .toList();
    }

    @Transactional
    public ProjectArtifactResponse storeInputArtifact(UUID projectId, ProjectArtifactType type, MultipartFile file) {
        ensureProjectExists(projectId);

        /*
         * Reject incompatible network input formats before writing any new file to the project workspace.
         */
        if (type == ProjectArtifactType.DSG_INPUT || type == ProjectArtifactType.TPO_INPUT) {

            var filename = file.getOriginalFilename() == null ? "" : file.getOriginalFilename().toLowerCase(java.util.Locale.ROOT);
            var expectedExtension = type == ProjectArtifactType.DSG_INPUT ? ".dsg" : ".tpo";

            if (!filename.endsWith(expectedExtension)) {
                throw new IllegalArgumentException("The selected file must have the " + expectedExtension + " extension.");
            }
            sourcePolicy.assertCanStage(projectId, type);
        }

        try {
            var original = safeFilename(file.getOriginalFilename());
            var version = isNetworkInput(type)
                    ? initialRevisionNumber(original)
                    : artifactRepository
                      .countByProjectIdAndType(projectId, type) + 1;
            var stored = versionedFilename(original, version);

            var projectDir = projectInputDir(projectId);
            Files.createDirectories(projectDir);

            var target = projectDir.resolve(stored).normalize();
            if (!target.startsWith(projectDir)) {
                throw new IllegalArgumentException("Invalid artifact path.");
            }

            file.transferTo(target);

            var artifact = ProjectArtifactEntity.builder()
                    .projectId(projectId)
                    .type(type)
                    .originalFilename(original)
                    .storedFilename(stored)
                    .relativePath(rootDir().relativize(target).toString())
                    .sizeBytes(Files.size(target))
                    .version(version)
                    .generated(false)
                    .build();

            var saved = artifactRepository.save(artifact);
            /*
             * A newly uploaded DSG/TPO starts a new, independent revision family.
             */
            if (isNetworkInput(type) && saved.getRevisionFamilyId() == null) {
                saved.setRevisionFamilyId(saved.getId());
                saved = artifactRepository.save(saved);
            }

            return mapper.toResponse(saved);
        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not store artifact.", exception);
        }
    }

    @Transactional
    public ProjectArtifactResponse registerGeneratedArtifact(UUID projectId, ProjectArtifactType type, String originalFilename, byte[] content) {
        ensureProjectExists(projectId);

        try {
            var version = artifactRepository.countByProjectIdAndType(projectId, type) + 1;
            var original = safeFilename(originalFilename);
            var stored = versionedFilename(original, version);

            var outputDir = projectOutputDir(projectId);
            Files.createDirectories(outputDir);

            var target = outputDir.resolve(stored).normalize();
            Files.write(target, content);

            var artifact = ProjectArtifactEntity.builder()
                    .projectId(projectId)
                    .type(type)
                    .originalFilename(original)
                    .storedFilename(stored)
                    .relativePath(rootDir().relativize(target).toString())
                    .sizeBytes(Files.size(target))
                    .version(version)
                    .generated(true)
                    .build();

            return mapper.toResponse(artifactRepository.save(artifact));
        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not register generated artifact.", exception);
        }
    }

    @Transactional(readOnly = true)
    public Resource loadArtifact(UUID projectId, UUID artifactId) {
        ensureProjectExists(projectId);

        var artifact = artifactRepository.findById(artifactId)
                .orElseThrow(() -> new ResourceNotFoundException("Artifact not found: " + artifactId));

        if (!artifact.getProjectId().equals(projectId)) {
            throw new ResourceNotFoundException("Artifact not found in project: " + artifactId);
        }

        var file = rootDir().resolve(artifact.getRelativePath()).normalize();
        if (!file.startsWith(rootDir()) || !Files.exists(file)) {
            throw new ResourceNotFoundException("Artifact file is missing: " + artifactId);
        }

        return new FileSystemResource(file);
    }

    @Transactional(readOnly = true)
    public String readArtifactText(UUID projectId, UUID artifactId) {
        ensureProjectExists(projectId);

        var artifact = artifactRepository.findById(artifactId)
                .orElseThrow(() -> new ResourceNotFoundException("Artifact not found: " + artifactId));

        /*
         * Archived artifacts remain on disk for historical provenance, but they must not be reused as new domain inputs.
         */
        if (artifact.isArchived()) {
            throw new ResourceNotFoundException("Artifact is no longer active: " + artifactId);
        }

        if (!artifact.getProjectId().equals(projectId)){
            throw new ResourceNotFoundException("Artifact not found in project: " + artifactId);
        }

        var root = rootDir();
        var file = root.resolve(artifact.getRelativePath()).normalize();

        if (!file.startsWith(rootDir())) {
            throw new IllegalArgumentException("Invalid artifact path: " + file);
        }

        if (!Files.exists(file)) {
            throw new ResourceNotFoundException(
                    "Artifact file is missing on disk. artifactId=" + artifactId
                            + ", expectedPath=" + file
            );
        }

        if (!Files.isRegularFile(file)) {
            throw new IllegalArgumentException("Artifact path is not a regular file: " + file);
        }

        try {
            return Files.readString(file, StandardCharsets.UTF_8);
        } catch (MalformedInputException utf8Exception) {
            try {
                return Files.readString(file, StandardCharsets.ISO_8859_1);
            } catch (IOException fallbackException) {
                throw new IllegalArgumentException(
                        "Could not read artifact with UTF-8 or ISO-8859-1. path=" + file,
                        fallbackException
                );
            }
        } catch (Exception e){
            throw new IllegalArgumentException("Could not read artifact file. path=" + file,
                    e);
        }
    }

    @Transactional(readOnly = true)
    public ProjectArtifactResponse latestArtifactOfType(UUID projectId, ProjectArtifactType type) {
        ensureProjectExists(projectId);

        return artifactRepository.findFirstByProjectIdAndTypeOrderByCreatedAtDesc(projectId, type)
                .map(mapper::toResponse).orElseThrow(() ->
                                new ResourceNotFoundException("No artifact of type " + type
                                                + " exists for project " + projectId)
                );
    }

    @Transactional(readOnly = true)
    public boolean hasArtifactType(UUID projectId, ProjectArtifactType type) {
        ensureProjectExists(projectId);

        return artifactRepository.findFirstByProjectIdAndTypeOrderByCreatedAtDesc(projectId, type).isPresent();
    }

    @Transactional(readOnly = true)
    public List<ProjectArtifactResponse> listLibraryArtifacts(ProjectArtifactType type) {
        if (!isReusableImportArtifact(type)) {
            throw new IllegalArgumentException("Artifact type is not available in the import library: " + type);
        }
        //
        return artifactRepository
                .findByTypeAndGeneratedFalseAndArchivedFalseOrderByCreatedAtDesc(type)
                .stream()
                .map(mapper::toResponse)
                .toList();
    }

    @Transactional
    public ProjectArtifactResponse copyInputArtifactToProject(UUID targetProjectId, UUID sourceArtifactId){
        ensureProjectExists(targetProjectId);

        var sourceArtifact = artifactRepository.findById(sourceArtifactId)
                .orElseThrow(() -> new ResourceNotFoundException("Artifact not found: " + sourceArtifactId));

        if (!isReusableImportArtifact(sourceArtifact.getType())) {
            throw new IllegalArgumentException("This artifact type cannot be copied into a project.");
        }
        if (sourceArtifact.isGenerated()){
            throw new IllegalArgumentException("Generated artifacts cannot be used as network input.");
        }
        /*
         * If the artifact already belongs to the selected project, there is no reason to duplicate it.
         */
        if(sourceArtifact.getProjectId().equals(targetProjectId)){
            return mapper.toResponse(sourceArtifact);
        }
        if (sourceArtifact.isArchived()) {
            throw new IllegalArgumentException("Archived artifacts cannot be copied to another project.");
        }
        try {
            var root = rootDir();
            var sourcePath = root.resolve(sourceArtifact.getRelativePath())
                    .normalize();
            if (!sourcePath.startsWith(root) || !Files.isRegularFile(sourcePath)) {
                throw new ResourceNotFoundException("Source artifact file is missing: " + sourceArtifactId);
        }

            var originalFilename = sourceArtifact.getOriginalFilename();
            var version = isNetworkInput(sourceArtifact.getType())
                    ? initialRevisionNumber(originalFilename)
                    : artifactRepository.countByProjectIdAndType(
                    targetProjectId, sourceArtifact.getType()) + 1;

            var storedFilename = versionedFilename(originalFilename, version);
            var targetDirectory = projectInputDir(targetProjectId);

            Files.createDirectories(targetDirectory);
            var targetPath = targetDirectory.resolve(storedFilename).normalize();

            if (!targetPath.startsWith(targetDirectory)) {
                throw new IllegalArgumentException("Invalid target artifact path.");
            }

            sourcePolicy.assertCanStage(targetProjectId, sourceArtifact.getType());
            Files.copy(sourcePath, targetPath);

            var copiedArtifact = ProjectArtifactEntity.builder()
                            .projectId(targetProjectId)
                            .type(sourceArtifact.getType())
                            .originalFilename(originalFilename)
                            .storedFilename(storedFilename)
                            .relativePath(root.relativize(targetPath).toString())
                            .sizeBytes(Files.size(targetPath))
                            .version(version)
                            .generated(false)
                            .build();

            var saved = artifactRepository.save(copiedArtifact);
            if (isNetworkInput(sourceArtifact.getType()) && saved.getRevisionFamilyId() == null) {
                saved.setRevisionFamilyId(saved.getId());
                saved = artifactRepository.save(saved);
            }

            return mapper.toResponse(saved);

        } catch (ResourceNotFoundException exception) {
            throw exception;
        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not copy artifact into project.", exception);
        }
    }

    @Transactional(readOnly = true)
    public TpoArtifactRequirementsResponse inspectTpoRequirements(UUID artifactId) {
        var artifact = artifactRepository.findById(artifactId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Artifact not found: " + artifactId)
                );

        if (artifact.getType() != ProjectArtifactType.TPO_INPUT) {
            throw new IllegalArgumentException("Artifact is not a TPO input file.");
        }

        if (artifact.isGenerated()) {
            throw new IllegalArgumentException("Generated artifacts cannot be used as TPO input.");
        }

        /*
         * Read the file using its own project ownership.
         * This also keeps the filesystem protection already
         * implemented by readArtifactText().
         */
        var content = readArtifactText(artifact.getProjectId(), artifactId);
        var hasDiameters = hasSection(content, "Diameters");
        var hasOrifices = hasSection(content, "Orifices");

        return new TpoArtifactRequirementsResponse(artifactId, hasDiameters, hasOrifices, !hasDiameters);
    }

    @Transactional
    public void archiveDatabaseArtifact(UUID artifactId) {
        var artifact = artifactRepository.findById(artifactId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Artifact not found: " + artifactId));

        boolean isDatabase =
                artifact.getType() == ProjectArtifactType.DB_DIAMETERS_INPUT
                        ||
                        artifact.getType() == ProjectArtifactType.DB_ORIFICES_INPUT;

        if (!isDatabase || artifact.isGenerated()) {
            throw new IllegalArgumentException("Only input database artifacts can be archived.");}
        artifact.setArchived(true);

        artifactRepository.save(artifact);
    }

    /**
     * Delete the complete filesystem workspace owned by one project.
     *
     * <p>The method removes files before directories and never allows deletion
     * outside the configured NeatWork workspace root.</p>
     *
     * @param projectId project whose workspace must be removed
     */
    public void deleteProjectWorkspace(UUID projectId) {
        var root = rootDir();
        var projectDir = root.resolve(projectId.toString()).normalize();

        if (!projectDir.startsWith(root)) {
            throw new IllegalArgumentException("Invalid project workspace path");
        }

        if (!Files.exists(projectDir)) {
            return;
        }

        try (var paths = Files.walk(projectDir)) {
            var orderedPaths = paths.sorted(Comparator.reverseOrder()).toList();

            for (var path : orderedPaths) {
                Files.deleteIfExists(path);
            }
        } catch (IOException exception) {
            throw new IllegalStateException("Could not delete project workspace: " + projectId, exception);
        }
    }


    /**
     * Removes one DSG or TPO artifact from the active project library without deleting its physical file.
     * Historical snapshots and simulation results remain intact. The project is locked to prevent a concurrent import from using the artifact while it is archived.
     * @param projectId owning project identifier
     * @param artifactId artifact requested for deletion
     */
    @Transactional
    public void archiveNetworkInput(UUID projectId, UUID artifactId) {
        /*
         * Serialize changes to project input files with the domain-import workflow.
         */
        projectRepository.findByIdForUpdate(projectId).orElseThrow(() ->
                        new ResourceNotFoundException("Project not found: " + projectId));

        var artifact = artifactRepository
                .findByIdAndProjectId(artifactId, projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Artifact not found in project: " + artifactId));

        /*
         * Only primary network input files can be removed through this endpoint.
         */
        boolean isNetworkInput = artifact.getType() == ProjectArtifactType.DSG_INPUT
                        ||
                        artifact.getType() == ProjectArtifactType.TPO_INPUT;

        if (!isNetworkInput || artifact.isGenerated()) {
            throw new IllegalArgumentException("Only DSG and TPO input files " + "can be deleted here.");
        }

        /*
         * Make repeated deletion requests harmless.
         */
        if (artifact.isArchived()) {
            return;
        }

        /*
         * Keep the physical file and metadata for historical snapshots and simulations.
         */
        artifact.setArchived(true);
        artifact.setArchivedAt(java.time.Instant.now());

        artifactRepository.save(artifact);
    }

    /**
     * Creates a new immutable revision of one DSG/TPO input artifact.
     * The source artifact is never overwritten.
     * @param projectId project owner
     * @param source source artifact
     * @param customFilename optional user-defined filename
     * @param content validated serialized file content
     * @return newly persisted revision
     */
    @Transactional
    public ProjectArtifactEntity registerNetworkRevision(UUID projectId, ProjectArtifactEntity source, String customFilename, byte[] content) {
        ensureProjectExists(projectId);

        if (!source.getProjectId().equals(projectId)) {
            throw new IllegalArgumentException("Source artifact does not belong to project.");
        }
        if (source.getType() != ProjectArtifactType.DSG_INPUT
                        &&
                        source.getType() != ProjectArtifactType.TPO_INPUT
        ) {
            throw new IllegalArgumentException("Only DSG and TPO files support revisions.");
        }
        if (source.isArchived()) {
            throw new IllegalArgumentException("Archived files cannot be edited.");
        }

        var familyId = source.getRevisionFamilyId() != null ? source.getRevisionFamilyId() : source.getId();
        var latest = artifactRepository
                        .findFirstByProjectIdAndRevisionFamilyIdOrderByVersionDesc(
                                projectId, familyId).orElse(source);

        var nextVersion = latest.getVersion() + 1;
        var extension = source.getType() == ProjectArtifactType.DSG_INPUT ? ".dsg" : ".tpo";
        var visibleFilename = customFilename == null
                        ? defaultRevisionFilename(source.getOriginalFilename(), nextVersion, extension)
                        : validateCustomRevisionFilename(customFilename, extension);

        try {
            var inputDir = projectInputDir(projectId);
            Files.createDirectories(inputDir);

            /*
             * The internal storage filename is intentionally different from the user-visible filename.
             * This prevents collisions when custom filenames are reused across multiple revisions.
             */
            var storedFilename = internalRevisionFilename(visibleFilename, nextVersion);
            var target = inputDir.resolve(storedFilename).normalize();
            if (!target.startsWith(inputDir)) {
                throw new IllegalArgumentException("Invalid revision path.");
            }

            Files.write(target, content);
            var revision = ProjectArtifactEntity.builder()
                            .projectId(projectId)
                            .type(source.getType())
                            .originalFilename(visibleFilename)
                            .storedFilename(storedFilename)

                            .relativePath(rootDir().relativize(target).toString())
                            .sizeBytes(Files.size(target))
                            .version(nextVersion)
                            .generated(false)
                            .archived(false)
                            .parentArtifactId(source.getId())
                            .revisionFamilyId(familyId)
                            .build();


            return artifactRepository.save(revision);

        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not save input revision.", exception);
        }
    }

    /**
     * Returns true only for editable primary network files.
     */
    private boolean isNetworkInput(ProjectArtifactType type) {
        return type == ProjectArtifactType.DSG_INPUT || type == ProjectArtifactType.TPO_INPUT;
    }

    /**
     * Reads a trailing "_vN" suffix when the imported filename already contains a human revision number.
     */
    private int initialRevisionNumber(String filename) {
        var dot = filename.lastIndexOf('.');
        var baseName = dot >= 0 ? filename.substring(0, dot) : filename;
        var matcher = java.util.regex.Pattern.compile("(?i)_v([0-9]+)$").matcher(baseName);
        if (!matcher.find()) {
            return 1;
        }

        try {
            return Integer.parseInt(matcher.group(1));
        } catch (NumberFormatException ignored) {
            return 1;
        }
    }

    /**
     * Creates the standard automatic filename:
     * Toy_v1.dsg -> Toy_v2.dsg
     */
    private String defaultRevisionFilename(String sourceFilename, int version, String extension) {
        var filename = sourceFilename.trim();
        var dot = filename.lastIndexOf('.');
        var base = dot >= 0 ? filename.substring(0, dot) : filename;
        base = base.replaceFirst("(?i)_v[0-9]+$", "");
        return base + "_v" + version + extension;
    }

    /**
     * Validates a user-defined visible filename.
     * The extension is appended automatically when omitted.
     * A conflicting extension is rejected.
     */
    private String validateCustomRevisionFilename(String requestedFilename, String expectedExtension) {
        if (requestedFilename == null || requestedFilename.isBlank()) {
            throw new IllegalArgumentException("A custom filename is required.");
        }
        var filename = Normalizer.normalize(requestedFilename.trim(), Normalizer.Form.NFKC);

        if (filename.contains("/") || filename.contains("\\") || filename.equals(".") || filename.equals("..")) {
            throw new IllegalArgumentException("Invalid filename.");
        }
        if (!filename.contains(".")) {
            filename += expectedExtension;
        }
        if (!filename.toLowerCase().endsWith(expectedExtension)) {
            throw new IllegalArgumentException("The filename must use " + expectedExtension);
        }
        if (filename.length() > 240) {
            throw new IllegalArgumentException("Filename is too long.");
        }

        return filename;
    }

    private String internalRevisionFilename(String visibleFilename, int version) {
        var dot = visibleFilename.lastIndexOf('.');
        var base = dot >= 0 ? visibleFilename.substring(0, dot) : visibleFilename;
        var extension = dot >= 0 ? visibleFilename.substring(dot) : "";

        var safeBase = safeFilename(base);
        var unique = UUID.randomUUID().toString().substring(0, 8);

        return safeBase + "_rev" + version + "_" + unique + extension;
    }

    private boolean hasSection(String content, String sectionName) {
        var expected = "!" + sectionName.toLowerCase();

        return content.lines().map(String::trim).map(String::toLowerCase)
                .anyMatch(line -> line.equals(expected));
    }

    private boolean isReusableImportArtifact(ProjectArtifactType type) {
        return type == ProjectArtifactType.DSG_INPUT || type == ProjectArtifactType.TPO_INPUT
                || type == ProjectArtifactType.DB_DIAMETERS_INPUT || type == ProjectArtifactType.DB_ORIFICES_INPUT;
    }

    private void ensureProjectExists(UUID projectId) {
        if (!projectRepository.existsById(projectId)) {
            throw new ResourceNotFoundException("Project not found: " + projectId);
        }
    }

    private Path rootDir() {
        return Path.of(workspaceProperties.rootDir()).toAbsolutePath().normalize();
    }

    private Path projectInputDir(UUID projectId) {
        return rootDir().resolve(projectId.toString()).resolve("inputs").normalize();
    }

    private Path projectOutputDir(UUID projectId) {
        return rootDir().resolve(projectId.toString()).resolve("outputs").normalize();
    }

    private String safeFilename(String filename) {
        var value = filename == null || filename.isBlank() ? "artifact.bin" : filename;
        value = Normalizer.normalize(value, Normalizer.Form.NFKC);
        return value.replaceAll("[^a-zA-Z0-9._-]", "_");
    }

    private String versionedFilename(String filename, int version) {
        var dot = filename.lastIndexOf(".");
        if (dot < 0) {
            return filename + "_v" + version;
        }

        return filename.substring(0, dot) + "_v" + version + filename.substring(dot);
    }
}