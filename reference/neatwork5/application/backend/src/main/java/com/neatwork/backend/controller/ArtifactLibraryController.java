package com.neatwork.backend.controller;

import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.response.ProjectArtifactResponse;
import com.neatwork.backend.response.TpoArtifactRequirementsResponse;
import com.neatwork.backend.service.ProjectArtifactService;
import lombok.RequiredArgsConstructor;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.UUID;

@RestController
@RequestMapping("/api/artifacts")
@RequiredArgsConstructor
public class ArtifactLibraryController {

    private final ProjectArtifactService projectArtifactService;

    @GetMapping
    public List<ProjectArtifactResponse> listArtifacts(@RequestParam ProjectArtifactType type) {
        return projectArtifactService.listLibraryArtifacts(type);
    }

    @GetMapping("/{artifactId}/tpo-requirements")
    public TpoArtifactRequirementsResponse
    getTpoRequirements(@PathVariable UUID artifactId) {
        return projectArtifactService.inspectTpoRequirements(artifactId);
    }

    @DeleteMapping("{artifactId}/library")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    public void removeFromLibrary(@PathVariable UUID artifactId){
        projectArtifactService.archiveDatabaseArtifact(artifactId);
    }
}