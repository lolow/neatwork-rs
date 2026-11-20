package com.neatwork.backend.service;

import tools.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.assertj.core.api.Assertions.assertThatCode;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class DomainCatalogIntegrityServiceTest {

    private DomainCatalogIntegrityService service;


    @BeforeEach
    void setUp() {
        service = new DomainCatalogIntegrityService(new ObjectMapper());
    }

    private Map<String, Object> domain(
            List<Map<String, Object>> diameters,
            List<Map<String, Object>> pipes,
            List<Map<String, Object>> orifices,
            List<Map<String, Object>> nodes
    ) {

        return Map.of("tables", Map.of("diameters", diameters,
                "pipes", pipes, "orifices", orifices, "nodes", nodes));
    }

    @Test
    void acceptsExistingPipeDiameterReferences() {
        var domain = domain(
                List.of(Map.of("ID", "D1")),
                List.of(Map.of("id", "A1", "ID1", "D1", "lon2", 0)),
                List.of(), List.of());

        assertThatCode(() -> service.validate(domain)).doesNotThrowAnyException();
    }

    @Test
    void rejectsMissingPipeDiameterReference() {
        var domain = domain(List.of(Map.of("ID", "D1")),
                List.of(Map.of("id", "A1", "ID1", "D99", "lon2", 0)),
                List.of(), List.of());

        assertThatThrownBy(() -> service.validate(domain))
                .isInstanceOf(RuntimeException.class)
                .hasMessageContaining("'d99'");
    }

    @Test
    void ignoresInactiveSecondSegmentDiameter() {
        var domain = domain(List.of(Map.of("ID", "D1")),
                List.of(Map.of("id", "A1", "ID1", "D1", "lon2", 0, "ID2", "D99")),
                List.of(), List.of());

        assertThatCode(() -> service.validate(domain)).doesNotThrowAnyException();
    }

    @Test
    void rejectsTerminalOrificeMissingFromCatalog() {
        var domain = domain(List.of(), List.of(), List.of(Map.of("diameter_m", 0.003)),
                List.of(Map.of("noeud", "P3", "statut", 2, "orif_com", 0.005)));

        assertThatThrownBy(() -> service.validate(domain))
                .isInstanceOf(RuntimeException.class)
                .hasMessageContaining("P3");
    }
}