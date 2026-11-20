package com.neatwork.backend.model.mapper;

import com.neatwork.backend.model.entity.ProjectArtifactEntity;
import com.neatwork.backend.response.ProjectArtifactResponse;
import org.springframework.stereotype.Component;

@Component
public class ProjectArtifactMapper {

    public ProjectArtifactResponse toResponse(ProjectArtifactEntity artifact) {
        return new ProjectArtifactResponse(
                artifact.getId(),
                artifact.getProjectId(),
                artifact.getType(),
                artifact.getOriginalFilename(),
                artifact.getStoredFilename(),
                artifact.getSizeBytes(),
                artifact.getVersion(),
                artifact.isGenerated(),
                artifact.isArchived(),

                artifact.getParentArtifactId(),
                artifact.getRevisionFamilyId(),
                artifact.getCreatedAt()
        );
    }
}
