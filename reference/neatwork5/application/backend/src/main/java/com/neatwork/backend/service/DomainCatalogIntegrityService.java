package com.neatwork.backend.service;

import com.neatwork.backend.exception.DomainIntegrityException;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;

import tools.jackson.databind.JsonNode;
import tools.jackson.databind.ObjectMapper;

import java.util.*;


@Service
@RequiredArgsConstructor
public class DomainCatalogIntegrityService {
    //private static final double ORIFICE_ABSOLUTE_TOLERANCE = 1.0e-9;

    private final ObjectMapper objectMapper;

    //Validates catalog references before one edited domain is persisted as a new immutable revision.
    public void validate(Map<String, Object> domain) {
        JsonNode root = objectMapper.valueToTree(domain);
        JsonNode tables = root.path("tables");
        validatePipeDiameters(tables);
        //validateOrificeDiameters(tables);
        validateClientBehaviors(tables);
    }

    private String normalizeKey(String value) {
        if (value == null) {
            return "";
        }

        return value.trim().toLowerCase(Locale.ROOT).replaceAll("[\\s_-]+", "");
    }

    private JsonNode findArray(JsonNode tables, String... aliases) {
        if (tables == null || !tables.isObject()) {
            return null;
        }

        Set<String> expected = new HashSet<>();
        for (String alias : aliases) {
            expected.add(normalizeKey(alias));
        }
        for (var entry : tables.properties()) {
            if (expected.contains(normalizeKey(entry.getKey())) && entry.getValue().isArray()) {
                return entry.getValue();
            }
        }
        return null;
    }

    private JsonNode readField(JsonNode row, String... candidates) {
        if (row == null || !row.isObject()) {
            return null;
        }

        Set<String> expected = new HashSet<>();
        for (String candidate : candidates) {
            expected.add(normalizeKey(candidate));
        }

        for (var entry : row.properties()) {
            if (expected.contains(normalizeKey(entry.getKey()))) {
                return entry.getValue();
            }
        }

        return null;
    }

    private String referenceId(JsonNode value) {
        if (value == null || value.isNull()) {
            return null;
        }

        String text = value.asString("").trim();
        if (text.isBlank() || text.equalsIgnoreCase("N") || text.equals("-")) {
            return null;
        }

        return text.toLowerCase(Locale.ROOT);
    }

    private void validatePipeDiameters(JsonNode tables) {
        JsonNode diameters = findArray(tables,
                "diameters",
                "diameter_list",
                "diameterlist",
                "pipe_diameters",
                "pipe_diameter_database"
        );

        JsonNode pipes = findArray(tables,
                "pipes",
                "pipe_list",
                "pipelist",
                "arcs",
                "edges"
        );

        if (pipes == null) {
            return;
        }

        Set<String> available = new HashSet<>();
        if (diameters != null) {
            for (JsonNode row : diameters) {
                String id = referenceId(readField(row,
                                "ID",
                                "id",
                                "diameter_id",
                                "Nominal"
                ));

                if (id != null) {
                    available.add(id);
                }
            }
        }

        int index = 0;
        for (JsonNode pipe : pipes) {
            validatePipeReference(pipe, index, "Segment 1",
                    readField(pipe, "ID1", "id1", "segment1DiameterId"),
                    true, available
            );
            JsonNode lon2Node = readField(pipe, "lon2", "segment2Length", "length2");

            double lon2 = lon2Node == null ? 0.0 : lon2Node.asDouble(0.0);

            validatePipeReference(pipe, index, "Segment 2", readField(pipe, "ID2", "id2", "segment2DiameterId"),
                    Math.abs(lon2) > 1.0e-12, available
            );

            index++;
        }
    }

    private void validatePipeReference(JsonNode pipe, int index, String segment, JsonNode value, boolean active, Set<String> available) {
        if (!active) {
            return;
        }

        String id = referenceId(value);
        if (id == null) {
            return;
        }
        if (available.contains(id)) {
            return;
        }
        JsonNode from = readField(pipe, "pred", "from");

        JsonNode to = readField(pipe, "succ", "to");
        String pipeLabel = from != null && to != null
                        ? (from.asString() + " → " + to.asString())
                        : ("row " + (index + 1));

        throw new DomainIntegrityException("Invalid pipe diameter reference: " + segment
                + " of " + pipeLabel
                + " uses '" + id
                + "', but that diameter does not exist " + "in the pipe diameter catalog."
        );
    }

   /* private void validateOrificeDiameters(JsonNode tables) {
        JsonNode orifices = findArray(tables,
                "orifices",
                "orifice_list",
                "orificelist",
                "orifice_diameters",
                "orifice_diameter_database"
        );

        JsonNode nodes = findArray(tables, "nodes", "node_list", "nodelist");
        if (nodes == null || orifices == null) {
            return;
        }

        Set<Double> available = new HashSet<>();

        for (JsonNode row : orifices) {
            Double diameter = orificeDiameterMetres(row);
            if (diameter != null) {
                available.add(diameter);
            }
        }

        int index = 0;
        for (JsonNode node : nodes) {
            int status = integerValue(readField(node, "statut", "status", "statusCode"), -1);
            if (status != 2) {
                index++;
                continue;
            }

            validateOrificeReference(node, index, "Ideal orifice", numericValue(readField(node,
                            "orif_ideal",
                            "idealOrifice",
                            "idealOrificeDiameterM",
                            "ideal_orifice_diameter_m")),
                    available
            );

            validateOrificeReference(node, index, "Commercial orifice", numericValue(readField(node,
                            "orif_com",
                            "commercialOrifice",
                            "commercialOrificeDiameterM",
                            "commercial_orifice_diameter_m")),
                    available
            );

            index++;
        }
    }
*/

    /*private Double orificeDiameterMetres(JsonNode row) {
        Double metres = numericValue(
                readField(row, "diameter_m", "diameterM", "diameter", "in_diam", "inDiam"));

        if (metres != null) {
            return metres;
        }

        Double millimetres = numericValue(readField(row, "diameter_mm", "diameterMm"));
        if (millimetres != null) {
            return (millimetres / 1000.0);
        }
        return null;
    }
*/

    private Double numericValue(JsonNode value) {
        if (value == null || value.isNull()) {
            return null;
        }
        if (value.isNumber()) {
            return value.asDouble();
        }
        String text = value.asString(null);

        if (text == null || text.isBlank()) {
            return null;
        }
        try {
            return Double.parseDouble(text.trim());
        } catch (NumberFormatException exception) {
            return null;
        }
    }

    private int integerValue(JsonNode value, int fallback) {
        Double numeric = numericValue(value);

        return numeric == null ? fallback : numeric.intValue();
    }

   /* private boolean containsDiameter(Set<Double> available, double requested) {
        for (double candidate : available) {
            double scale = Math.max(1.0, Math.max(Math.abs(candidate), Math.abs(requested)));

            double tolerance = Math.max(ORIFICE_ABSOLUTE_TOLERANCE, 1.0e-7 * scale);
            if (Math.abs(candidate - requested) <= tolerance) {
                return true;
            }
        }

        return false;
    }
*/

    /*private void validateOrificeReference(JsonNode node, int index, String usage, Double diameter, Set<Double> available) {
        // Zero means that no terminal orifice is assigned.
        if (diameter == null || Math.abs(diameter) <= 1.0e-12) {
            return;
        }
        if (containsDiameter(available, diameter)) {
            return;
        }

        JsonNode idNode = readField(node, "noeud", "node", "id");
        String nodeLabel = idNode == null ? ("row " + (index + 1)) : idNode.asString();

        throw new DomainIntegrityException("Invalid orifice catalog reference: " + nodeLabel
                + " uses " + usage
                + " diameter " + diameter
                + " m, but that diameter does not exist " + "in the orifice catalog."
        );
    }
*/

    private void validateClientBehaviors(JsonNode tables) {
        JsonNode behaviors = findArray(tables, "clientBehaviors", "client_behaviors");
        JsonNode clients = findArray(tables, "clients", "client_list", "clientlist");
        Set<String> available = new HashSet<>();
        if (behaviors != null) {
            for (JsonNode behavior : behaviors) {
                String id = referenceId(readField(behavior, "behaviorId", "behavior_id"));

                if (id == null) {
                    continue;
                }

                if (!available.add(id)) {
                    throw new DomainIntegrityException("Duplicate client behavior ID: " + id + ".");
                }
                Double probability = numericValue(readField(behavior, "openProbability", "open_probability"));

                if (probability == null || probability < 0.0 || probability > 1.0) {
                    throw new DomainIntegrityException("Client behavior " + id + " has an invalid opening probability.");
                }

                Double targetFlow = numericValue(readField(behavior, "targetFlowLps", "target_flow_lps"));

                if (targetFlow == null || targetFlow < 0.0) {
                    throw new DomainIntegrityException("Client behavior " + id + " has an invalid target flow.");
                }
            }
        }

        if (clients == null) {
            return;
        }

        for (JsonNode client : clients) {
            String behaviorId = referenceId(readField(client, "behaviorId", "behavior_id"));

            // Legacy clients remain valid.
            if (behaviorId == null) {
                continue;
            }

            if (!available.contains(behaviorId)) {
                String clientId = Optional.ofNullable(readField(client, "client", "clientId", "client_id"))
                        .map(JsonNode::asString)
                        .orElse("?");

                throw new DomainIntegrityException("Client "
                                + clientId + " references behavior \""
                                + behaviorId + "\" but no such client behavior exists.");
            }
        }
    }
}
