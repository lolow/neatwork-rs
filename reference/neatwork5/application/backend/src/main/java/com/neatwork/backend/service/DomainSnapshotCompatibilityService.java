package com.neatwork.backend.service;

import com.neatwork.backend.enums.ComparisonCompatibilityLevel;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.response.ComparableEntitySummary;
import com.neatwork.backend.response.DomainSnapshotCompatibilityResponse;
import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.response.DomainComparisonChangeSummary;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.*;
import java.util.stream.Collectors;

/**
 * Compares two domain snapshots belonging to the same project.
 *
 * <p>Security note: project ownership is always enforced through
 * {@link ProjectDomainSnapshotRepository#findByIdAndProjectId(UUID, UUID)} before
 * any compatibility decision is returned to the caller.</p>
 */
@Service
@RequiredArgsConstructor
public class DomainSnapshotCompatibilityService {

    private final ProjectDomainSnapshotRepository snapshotRepository;

    /**
     * Stable identity of a node: its id plus its role/status code.
     */
    private record NodeIdentity(String id, int role) {
    }

    /**
     * Stable identity of an edge (pipe): its id plus its endpoints.
     */
    private record EdgeIdentity(String id, String from, String to) {
    }

    /**
     * Stable identity of a client: its id plus the terminal it is attached to.
     */
    private record ClientIdentity(String id, String terminalId) {
    }

    /**
     * Derived, comparable view of a snapshot's domain JSON.
     * All fields are order-independent (sets/fingerprints) so two snapshots with equivalent content produce identical profiles.
     */
    private record SnapshotProfile(
            Set<NodeIdentity> nodes,
            Set<EdgeIdentity> edges,
            Set<ClientIdentity> clients,
            Set<String> sources,
            Set<String> terminals,
            String topologyFingerprint,
            String demandFingerprint,
            String effectiveInputFingerprint
    ) {
    }

    // ---------------------------------------------------------------------
    // Generic key/value helpers
    // ---------------------------------------------------------------------

    /**
     * Normalizes a key for tolerant alias matching (lowercase, no separators).
     */
    private String normalizeKey(String value) {
        if (value == null) {
            return "";
        }

        return value.trim().toLowerCase(Locale.ROOT).replaceAll("[\\s_-]+", "");
    }

    /**
     * Reads the first non-null value whose key matches any of the given aliases.
     */
    private Object readValue(Map<String, Object> row, String... aliases) {

        Set<String> normalizedAliases =
                Arrays.stream(aliases).map(this::normalizeKey).collect(Collectors.toSet());

        for (var entry : row.entrySet()) {
            if (normalizedAliases.contains(normalizeKey(entry.getKey()))) {
                return entry.getValue();
            }
        }

        return null;
    }

    // ---------------------------------------------------------------------
    // Row -> identity mappers
    // ---------------------------------------------------------------------

    /**
     * Extracts the node id from a node row.
     */
    private String nodeId(Map<String, Object> row) {
        Object value = readValue(row, "noeud", "node", "nodeId", "node_id", "id");

        return value == null ? null : String.valueOf(value).trim();
    }

    /**
     * Extracts the integer node role/status, or -1 when absent/unparsable.
     */
    private int nodeRole(Map<String, Object> row) {
        Object value = readValue(row, "statut", "status", "statusCode", "nodeStatus");

        if (value == null) {
            return -1;
        }

        try {
            return Integer.parseInt(String.valueOf(value));
        } catch (NumberFormatException exception) {
            return -1;
        }
    }

    /**
     * Builds an {@link EdgeIdentity} from a pipe row.
     */
    private EdgeIdentity edgeIdentity(Map<String, Object> row) {
        String edgeId = textValue(readValue(row, "edgeId", "edge", "pipe", "id"));
        String from = textValue(readValue(row, "pred", "from", "predecessorId", "startNodeId"));
        String to = textValue(readValue(row, "succ", "to", "successorId", "endNodeId"));

        return new EdgeIdentity(edgeId, from, to);
    }

    /**
     * Null-safe, trimmed string conversion.
     */
    private String textValue(Object value) {
        if (value == null) {
            return "";
        }
        return String.valueOf(value).trim();
    }

    // ---------------------------------------------------------------------
    // Table extraction
    // ---------------------------------------------------------------------

    /**
     * Returns the list of row maps for a named table (with alias support).
     * Returns an empty list when the table is missing or malformed.
     */
    @SuppressWarnings("unchecked")
    private List<Map<String, Object>> tableRows(Map<String, Object> domain, String... aliases) {
        Object tablesValue = readValue(domain, "tables");

        if (!(tablesValue instanceof Map<?, ?> rawTables)) {
            return List.of();
        }

        Map<String, Object> tables = (Map<String, Object>) rawTables;
        Object tableValue = readValue(tables, aliases);
        if (!(tableValue instanceof List<?> rows)) {
            return List.of();
        }
        return rows.stream()
                .filter(Map.class::isInstance)
                .map(row -> (Map<String, Object>) row)
                .toList();
    }

    // ---------------------------------------------------------------------
    // Hashing / canonicalization
    // ---------------------------------------------------------------------

    /**
     * SHA-256 hex digest of a UTF-8 string.
     */
    private String sha256(String value) {
        try {
            var digest = MessageDigest.getInstance("SHA-256");
            byte[] bytes = digest.digest(value.getBytes(StandardCharsets.UTF_8));

            return HexFormat.of().formatHex(bytes);
        } catch (NoSuchAlgorithmException exception) {
            throw new IllegalStateException(exception);
        }
    }

    /**
     * Builds a {@link ClientIdentity} from a client row.
     */
    private ClientIdentity clientIdentity(Map<String, Object> row) {

        String id = textValue(readValue(row, "clientId", "client_id", "client", "id"));
        String terminalId = textValue(readValue(row, "terminalId", "terminal_id", "terminal", "nodeId"));

        return new ClientIdentity(id, terminalId);
    }

    // ---------------------------------------------------------------------
    // Profile construction
    // ---------------------------------------------------------------------

    /**
     * Builds a {@link SnapshotProfile} from a raw domain JSON map.
     * The resulting profile is stable under row reordering and carries
     * the three fingerprints used for compatibility decisions.
     */
    private SnapshotProfile buildProfile(Map<String, Object> domain) {

        List<Map<String, Object>> nodeRows = tableRows(domain, "nodes", "nodelist");
        List<Map<String, Object>> pipeRows = tableRows(domain, "pipes", "pipelist");
        List<Map<String, Object>> clientRows = tableRows(domain, "clients", "clientlist");

        // Nodes: keep only rows with a non-blank id.
        Set<NodeIdentity> nodes = nodeRows.stream()
                .map(row -> new NodeIdentity(nodeId(row), nodeRole(row)))
                .filter(node -> node.id() != null && !node.id().isBlank())
                .collect(Collectors.toSet());

        // Edges: keep only rows with a non-blank edge id.
        Set<EdgeIdentity> edges = pipeRows.stream()
                .map(this::edgeIdentity)
                .filter(edge -> !edge.id().isBlank())
                .collect(Collectors.toSet());

        // Clients: keep only rows with a non-blank client id.
        Set<ClientIdentity> clients = clientRows.stream()
                .map(this::clientIdentity)
                .filter(client -> !client.id().isBlank())
                .collect(Collectors.toSet());

        // Sources = nodes with role 0; Terminals = nodes with role 2.
        Set<String> sources = nodes.stream()
                .filter(node -> node.role() == 0)
                .map(NodeIdentity::id)
                .collect(Collectors.toSet());

        Set<String> terminals = nodes.stream()
                .filter(node -> node.role() == 2)
                .map(NodeIdentity::id)
                .collect(Collectors.toSet());

        return new SnapshotProfile(
                nodes,
                edges,
                clients,
                sources,
                terminals,
                topologyFingerprint(nodes, edges),
                demandStructureFingerprint(clients),
                sha256(canonicalizeForHash(buildEffectiveInputPayload(domain)))
        );
    }

    /**
     * Deterministic canonical string form of an arbitrary JSON-like value,
     * used as input to hashing so equal content always hashes identically.
     */
    private String canonicalizeForHash(Object value) {
        if (value == null) {
            return "null";
        }

        if (value instanceof Number number) {
            return canonicalNumber(number);
        }

        if (value instanceof Boolean booleanValue) {
            return booleanValue ? "true" : "false";
        }

        if (value instanceof Map<?, ?> map) {
            return map.entrySet()
                    .stream()
                    .sorted(Comparator.comparing(entry -> String.valueOf(entry.getKey())))
                    .map(entry -> String.valueOf(entry.getKey()) + "=" + canonicalizeForHash(entry.getValue()))
                    .collect(Collectors.joining("{", ",", "}"));
        }

        if (value instanceof Collection<?> collection) {
            return collection.stream()
                    .map(this::canonicalizeForHash)
                    .sorted()
                    .collect(Collectors.joining("[", ",", "]"));
        }

        return String.valueOf(value).trim();
    }

    /**
     * Fingerprint of the network topology (nodes with roles + edges with endpoints).
     */
    private String topologyFingerprint(Set<NodeIdentity> nodes, Set<EdgeIdentity> edges) {

        String nodePart = nodes.stream()
                .sorted(Comparator
                        .comparing(NodeIdentity::id)
                        .thenComparingInt(NodeIdentity::role))
                .map(node -> node.id() + "|" + node.role())
                .collect(Collectors.joining("\n"));

        String edgePart = edges.stream()
                .sorted(Comparator
                        .comparing(EdgeIdentity::id)
                        .thenComparing(EdgeIdentity::from)
                        .thenComparing(EdgeIdentity::to))
                .map(edge -> edge.id() + "|" + edge.from() + "|" + edge.to())
                .collect(Collectors.joining("\n"));

        return sha256(nodePart + "\n---EDGES---\n" + edgePart);
    }

    /**
     * Fingerprint of the client-to-terminal demand allocation.
     */
    private String demandStructureFingerprint(Set<ClientIdentity> clients) {

        String canonical = clients.stream()
                .sorted(Comparator.comparing(ClientIdentity::id).thenComparing(ClientIdentity::terminalId))
                .map(client -> client.id() + "|" + client.terminalId())
                .collect(Collectors.joining("\n"));

        return sha256(canonical);
    }

    /**
     * Extracts the "effective input" subset used for the input fingerprint
     * (params, technicalParams, designParams and normalized tables).
     */
    @SuppressWarnings("unchecked")
    private Map<String, Object> buildEffectiveInputPayload(Map<String, Object> domain) {

        var effective = new TreeMap<String, Object>();

        effective.put("params", domain.getOrDefault("params", Map.of()));
        effective.put("technicalParams", domain.getOrDefault("technicalParams", Map.of()));
        effective.put("designParams", domain.getOrDefault("designParams", Map.of()));
        Object tablesValue = domain.get("tables");
        Map<String, Object> normalizedTables = normalizeTablesForHash(tablesValue);

        if (tablesValue instanceof Map<?, ?> rawTables) {
            Map<String, Object> tables = new LinkedHashMap<>();

            for (var entry : rawTables.entrySet()) {
                tables.put(String.valueOf(entry.getKey()), entry.getValue());
            }

            /*
             * ClientBehavior IDs and display names are not hydraulic semantics. Replace the raw client / behavior representation with resolved demand rows.
             */
            normalizedTables.entrySet().removeIf(entry -> {
                String key = normalizeKey(entry.getKey());

                return Set.of("clients", "clientlist", "generatedclients", "clientbehaviors", "clientbehaviortypes").contains(normalizeKey(entry.getKey()));
            });

            normalizedTables.put("clients", buildEffectiveClientRows(tables));
        }

        effective.put("tables", normalizedTables);

        return effective;
    }

    /**
     * Canonical decimal representation of a number (no trailing zeros).
     */
    private String canonicalNumber(Number value) {

        return new BigDecimal(value.toString())
                .stripTrailingZeros()
                .toPlainString();
    }

    // ---------------------------------------------------------------------
    // Public API
    // ---------------------------------------------------------------------

    /**
     * Compares two snapshots of the same project.
     *
     * <p>Order of operations matters for security:</p>
     * <ol>
     *   <li>Load BOTH snapshots with {@code findByIdAndProjectId} so that a
     *       snapshot from another project cannot be used.</li>
     *   <li>Only after ownership is verified, short-circuit to
     *       {@code SAME_SNAPSHOT} when both ids are equal.</li>
     *   <li>Otherwise compute fingerprints and classify the comparison.</li>
     * </ol>
     */
    public DomainSnapshotCompatibilityResponse compare(UUID projectId, UUID leftSnapshotId, UUID rightSnapshotId) {
        // 1. Ownership check: both snapshots must belong to the requested project.
        var leftSnapshot = snapshotRepository
                .findByIdAndProjectId(leftSnapshotId, projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Left domain snapshot was not found."));

        var rightSnapshot = snapshotRepository
                .findByIdAndProjectId(rightSnapshotId, projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Right domain snapshot was not found."));

        // 2. Same-snapshot shortcut, allowed only now that ownership is verified.
        if (leftSnapshotId.equals(rightSnapshotId)) {
            // One profile is enough: both ids point to the same stored row.
            SnapshotProfile profile = buildProfile(leftSnapshot.getDomainJson());
            return sameSnapshotResponse(leftSnapshotId, rightSnapshotId, profile);
        }

        // 3. Build comparable profiles and fingerprints.
        var left = buildProfile(leftSnapshot.getDomainJson());
        var right = buildProfile(rightSnapshot.getDomainJson());

        boolean sameTopology = left.topologyFingerprint().equals(right.topologyFingerprint());

        boolean sameDemand = left.demandFingerprint().equals(right.demandFingerprint());

        boolean sameInput = left.effectiveInputFingerprint().equals(right.effectiveInputFingerprint());

        // 4. Classify compatibility level.
        ComparisonCompatibilityLevel level;

        if (sameInput) {
            level = ComparisonCompatibilityLevel.EQUIVALENT_INPUT;
        } else if (sameTopology) {
            level = ComparisonCompatibilityLevel.SAME_TOPOLOGY;
        } else {
            level = ComparisonCompatibilityLevel.TOPOLOGY_CHANGED;
        }

        // 5. Compute detailed change summary.
        DomainComparisonChangeSummary changes = buildChangeSummary(left, right);

        var terminalComparison = compareNodesByRoleAndContext(
                left.nodes(), left.edges(), right.nodes(), right.edges(), 2);


        var sourceComparison = compareNodesByRoleAndContext(
                left.nodes(), left.edges(), right.nodes(), right.edges(), 0);


        var transitComparison = compareNodesByRoleAndContext(
                left.nodes(), left.edges(), right.nodes(), right.edges(), 1);

        var pipeComparison = comparePipesByIdentity(left.edges(), right.edges());

        // 6. Build warnings and permission flags.
        List<String> warnings = new ArrayList<>();

        boolean directAllowed = level != ComparisonCompatibilityLevel.TOPOLOGY_CHANGED;

        boolean partialAllowed = true;

        if (level == ComparisonCompatibilityLevel.SAME_TOPOLOGY) {
            warnings.add("Input revision differs. Metric changes may reflect "
                    + "network parameters as well as simulation settings.");
        }

        if (!sameDemand) {
            warnings.add("Client-to-terminal allocation changed between revisions. "
                    + "Terminal statistics may reflect demand redistribution.");
        }

        if (level == ComparisonCompatibilityLevel.TOPOLOGY_CHANGED) {
            warnings.add("Network topology changed. Only matched entities "
                    + "should be compared directly.");
        }

        return new DomainSnapshotCompatibilityResponse(
                leftSnapshotId,
                rightSnapshotId,
                level,
                sameTopology,
                sameDemand,
                sameInput,
                directAllowed,
                partialAllowed,
                changes,
                List.copyOf(warnings),
                terminalComparison,
                sourceComparison,
                transitComparison,
                pipeComparison
        );
    }

    // ---------------------------------------------------------------------
    // Effective-input normalization
    // ---------------------------------------------------------------------

    /**
     * Normalizes the "tables" section for hashing: drops volatile
     * geometry (x/y/xdata/ydata), normalizes keys, and sorts rows so
     * semantically equal tables hash identically.
     */
    @SuppressWarnings("unchecked")
    private Map<String, Object> normalizeTablesForHash(Object tablesValue) {
        if (!(tablesValue instanceof Map<?, ?> rawTables)) {
            return Map.of();
        }
        var result = new TreeMap<String, Object>();
        for (var tableEntry : rawTables.entrySet()) {
            String tableName = String.valueOf(tableEntry.getKey());

            Object value = tableEntry.getValue();

            if (!(value instanceof List<?> rows)) {
                result.put(tableName, value);
                continue;
            }

            List<TreeMap<String, Object>> normalizedRows = rows.stream()
                    .filter(Map.class::isInstance)
                    .map(rawRow -> {
                        Map<String, Object> row = (Map<String, Object>) rawRow;
                        var normalized = new TreeMap<String, Object>();

                        for (var entry : row.entrySet()) {
                            String key = normalizeKey(entry.getKey());
                            if (key.equals("x") || key.equals("y") || key.equals("xdata") || key.equals("ydata")) {
                                continue;
                            }
                            normalized.put(entry.getKey(), entry.getValue());
                        }
                        return normalized;
                    })
                    .sorted(Comparator.comparing(this::canonicalizeForHash))
                    .toList();

            result.put(tableName, normalizedRows);
        }

        return result;
    }

    // ---------------------------------------------------------------------
    // Change summary
    // ---------------------------------------------------------------------

    /**
     * Computes the entity-level diff between two profiles:
     * node/pipe/terminal counts, matched counts, and added/removed/changed lists.
     */
    private DomainComparisonChangeSummary buildChangeSummary(SnapshotProfile left, SnapshotProfile right) {

        Map<String, NodeIdentity> leftNodes = left.nodes()
                .stream()
                .collect(Collectors.toMap(NodeIdentity::id, node -> node));

        Map<String, NodeIdentity> rightNodes = right.nodes()
                .stream()
                .collect(Collectors.toMap(NodeIdentity::id, node -> node));

        List<String> changedNodeRoles = leftNodes.keySet()
                .stream()
                .filter(rightNodes::containsKey)
                .filter(id -> leftNodes.get(id).role() != rightNodes.get(id).role())
                .sorted()
                .toList();

        List<String> addedNodes = rightNodes.keySet()
                .stream()
                .filter(id -> !leftNodes.containsKey(id))
                .sorted()
                .toList();

        List<String> removedNodes = leftNodes.keySet()
                .stream()
                .filter(id -> !rightNodes.containsKey(id))
                .sorted()
                .toList();

        int matchedNodeCount = (int) leftNodes.keySet()
                .stream()
                .filter(rightNodes::containsKey)
                .filter(id -> leftNodes.get(id).role() == rightNodes.get(id).role())
                .count();

        Map<String, EdgeIdentity> leftEdges = left.edges()
                .stream()
                .collect(Collectors.toMap(EdgeIdentity::id, edge -> edge));

        Map<String, EdgeIdentity> rightEdges = right.edges()
                .stream()
                .collect(Collectors.toMap(EdgeIdentity::id, edge -> edge));

        List<String> changedPipeEndpoints = leftEdges.keySet()
                .stream()
                .filter(rightEdges::containsKey)
                .filter(id -> {
                            EdgeIdentity leftEdge = leftEdges.get(id);
                            EdgeIdentity rightEdge = rightEdges.get(id);
                            return !Objects.equals(leftEdge.from(), rightEdge.from())
                                    ||
                                    !Objects.equals(leftEdge.to(), rightEdge.to());
                        }
                ).sorted()
                .toList();

        List<String> addedPipes = rightEdges.keySet()
                .stream()
                .filter(id -> !leftEdges.containsKey(id))
                .sorted()
                .toList();

        List<String> removedPipes = leftEdges.keySet()
                .stream()
                .filter(id -> !rightEdges.containsKey(id))
                .sorted()
                .toList();

        int matchedPipeCount = (int) leftEdges.keySet()
                .stream()
                .filter(rightEdges::containsKey)
                .filter(id -> {
                            EdgeIdentity leftEdge = leftEdges.get(id);
                            EdgeIdentity rightEdge = rightEdges.get(id);

                            return Objects.equals(leftEdge.from(), rightEdge.from())
                                    &&
                                    Objects.equals(leftEdge.to(), rightEdge.to());
                        }
                ).count();

        List<String> addedTerminals = right.terminals()
                .stream()
                .filter(id -> !left.terminals().contains(id))
                .sorted()
                .toList();

        List<String> removedTerminals =
                left.terminals()
                        .stream()
                        .filter(
                                id ->
                                        !right.terminals()
                                                .contains(id)
                        )
                        .sorted()
                        .toList();

        int matchedTerminalCount =
                (int) left.terminals()
                        .stream()
                        .filter(
                                right.terminals()::contains
                        )
                        .count();

        return new DomainComparisonChangeSummary(
                left.nodes().size(),
                right.nodes().size(),
                matchedNodeCount,
                addedNodes,
                removedNodes,
                changedNodeRoles,

                left.edges().size(),
                right.edges().size(),
                matchedPipeCount,
                addedPipes,
                removedPipes,
                changedPipeEndpoints,

                left.terminals().size(),
                right.terminals().size(),
                matchedTerminalCount,
                addedTerminals,
                removedTerminals
        );
    }

    // ---------------------------------------------------------------------
    // Same-snapshot response
    // ---------------------------------------------------------------------

    /**
     * Builds the response for the {@code SAME_SNAPSHOT} case.
     *
     * <p>The summary uses the real entity counts from the supplied profile
     * (instead of zeros) so downstream UIs see consistent numbers such as
     * {@code Nodes N -> N} and {@code Pipes M -> M}.</p>
     *
     * <p>Caller must have already verified project ownership for both ids.</p>
     */
    private DomainSnapshotCompatibilityResponse sameSnapshotResponse(UUID leftSnapshotId, UUID rightSnapshotId, SnapshotProfile profile) {

        var terminalComparison = allMatchedNodes(profile.nodes(), 2);

        var sourceComparison = allMatchedNodes(profile.nodes(), 0);

        var transitComparison = allMatchedNodes(profile.nodes(), 1);

        var pipeComparison = allMatchedPipes(profile.edges());
        return new DomainSnapshotCompatibilityResponse(
                leftSnapshotId,
                rightSnapshotId,
                ComparisonCompatibilityLevel.SAME_SNAPSHOT,

                true,   // same topology
                true,   // same demand
                true,   // same effective input
                true,   // direct comparison allowed
                true,   // partial comparison allowed

                new DomainComparisonChangeSummary(

                        // Nodes: left = right = matched = total, nothing added/removed/changed.
                        profile.nodes().size(),
                        profile.nodes().size(),
                        profile.nodes().size(),
                        List.of(),
                        List.of(),
                        List.of(),

                        // Pipes: left = right = matched = total, nothing added/removed/changed.
                        profile.edges().size(),
                        profile.edges().size(),
                        profile.edges().size(),
                        List.of(),
                        List.of(),
                        List.of(),

                        // Terminals: left = right = matched = total, nothing added/removed.
                        profile.terminals().size(),
                        profile.terminals().size(),
                        profile.terminals().size(),
                        List.of(),
                        List.of()
                ),

                List.of(),
                terminalComparison,
                sourceComparison,
                transitComparison,
                pipeComparison
        );
    }

    private ComparableEntitySummary allMatchedNodes(Set<NodeIdentity> nodes, int role) {
        List<String> matched = nodes.stream()
                        .filter(node -> node.role() == role)
                        .map(NodeIdentity::id)
                        .sorted().toList();

        return new ComparableEntitySummary(matched, List.of(), List.of(), List.of());
    }

    private ComparableEntitySummary allMatchedPipes(Set<EdgeIdentity> edges) {

        List<String> matched = edges.stream()
                        .map(EdgeIdentity::id)
                        .sorted()
                        .toList();

        return new ComparableEntitySummary(matched, List.of(), List.of(), List.of());
    }

    @SuppressWarnings("unchecked")
    private List<Map<String, Object>> buildEffectiveClientRows(Map<String, Object> tables) {
        Object behaviorsValue = readValue(tables, "clientBehaviors", "client_behaviors");
        Object clientsValue = readValue(tables, "clients", "client_list", "clientlist", "generated_clients");

        Map<String, Map<String, Object>> behaviorById = new HashMap<>();

        if (behaviorsValue instanceof List<?> behaviors) {
            for (Object raw : behaviors) {
                if (!(raw instanceof Map<?, ?> map)) {
                    continue;
                }

                Map<String, Object> row = (Map<String, Object>) map;
                String id = textValue(readValue(row, "behaviorId", "behavior_id"));

                if (id != null && !id.isBlank()) {
                    behaviorById.put(id, row);
                }
            }
        }

        List<Map<String, Object>> result = new ArrayList<>();

        if (clientsValue instanceof List<?> clients) {
            for (Object raw : clients) {
                if (!(raw instanceof Map<?, ?> map)) {
                    continue;
                }

                Map<String, Object> client = (Map<String, Object>) map;
                String clientId = textValue(readValue(client, "client", "clientId", "client_id"));

                String terminalId = textValue(readValue(client, "borne", "terminalId", "terminal_id"));
                String behaviorId = textValue(readValue(client, "behaviorId", "behavior_id"));

                Object probability;
                Object target;
                Object profile;

                if (behaviorId != null && !behaviorId.isBlank()) {
                    Map<String, Object> behavior = behaviorById.get(behaviorId);
                    if (behavior == null) {
                        var unresolved = new TreeMap<String, Object>();
                        unresolved.put("client", clientId);
                        unresolved.put("terminal", terminalId);
                        unresolved.put("unresolvedBehavior", behaviorId);
                        result.add(unresolved);
                        continue;
                    }

                    probability = readValue(behavior, "openProbability", "open_probability");
                    target = readValue(behavior, "targetFlowLps", "target_flow_lps");
                    profile = readValue(behavior, "profileId", "profile_id");
                } else {
                    probability = readValue(client, "proba", "openProbability", "open_probability");

                    target = readValue(client, "cible", "targetFlowLps", "target_flow_lps");
                    profile = null;
                }

                var effective = new TreeMap<String, Object>();

                effective.put("client", clientId);
                effective.put("terminal", terminalId);

                effective.put("probability", probability);
                effective.put("targetFlow", target);

                effective.put("profile", profile);
                result.add(effective);
            }
        }

        result.sort(Comparator.comparing(this::canonicalizeForHash));
        return result;
    }

    private record NodeContextIdentity(String nodeId, int role, List<String> incidentEdges) {
    }

    private String edgeContextKey(EdgeIdentity edge) {
        return edge.id() + "|" + edge.from() + "|" + edge.to();
    }

    private Map<String, List<String>> buildIncidentEdgeIndex(Set<EdgeIdentity> edges) {
        Map<String, List<String>> result = new HashMap<>();

        for (var edge : edges) {
            String key = edgeContextKey(edge);
            result.computeIfAbsent(edge.from(), ignored -> new ArrayList<>()).add(key);
            result.computeIfAbsent(edge.to(), ignored -> new ArrayList<>()).add(key);
        }

        result.values().forEach(Collections::sort);
        return result;
    }

    private NodeContextIdentity nodeContextIdentity(NodeIdentity node, Map<String, List<String>> incidentEdges) {
        return new NodeContextIdentity(node.id(), node.role(), List.copyOf(incidentEdges.getOrDefault(node.id(), List.of()))
        );
    }

    private ComparableEntitySummary compareNodesByRoleAndContext(
            Set<NodeIdentity> leftNodes,
            Set<EdgeIdentity> leftEdges,
            Set<NodeIdentity> rightNodes,
            Set<EdgeIdentity> rightEdges,
            int requiredRole
    ) {

        var leftIncident = buildIncidentEdgeIndex(leftEdges);
        var rightIncident = buildIncidentEdgeIndex(rightEdges);

        Map<String, NodeContextIdentity> leftById = leftNodes.stream().filter(node -> node.role() == requiredRole)
                .collect(Collectors.toMap(NodeIdentity::id, node -> nodeContextIdentity(node, leftIncident)));

        Map<String, NodeContextIdentity> rightById = rightNodes.stream().filter(node -> node.role() == requiredRole)
                .collect(Collectors.toMap(NodeIdentity::id, node -> nodeContextIdentity(node, rightIncident)));

        List<String> matched = new ArrayList<>();
        List<String> leftOnly = new ArrayList<>();
        List<String> rightOnly = new ArrayList<>();
        List<String> contextChanged = new ArrayList<>();
        for (var entry : leftById.entrySet()) {
            String id = entry.getKey();

            var right = rightById.get(id);

            if (right == null) {
                leftOnly.add(id);
                continue;
            }

            var left = entry.getValue();
            if (left.incidentEdges().equals(right.incidentEdges())) {
                matched.add(id);
            } else {
                contextChanged.add(id);
                leftOnly.add(id);
                rightOnly.add(id);
            }
        }

        for (String id : rightById.keySet()) {
            if (!leftById.containsKey(id)) {
                rightOnly.add(id);
            }
        }
        matched.sort(String::compareTo);
        leftOnly.sort(String::compareTo);
        rightOnly.sort(String::compareTo);
        contextChanged.sort(String::compareTo);

        return new ComparableEntitySummary(
                List.copyOf(matched),
                List.copyOf(leftOnly),
                List.copyOf(rightOnly),
                List.copyOf(contextChanged));
    }

    private ComparableEntitySummary comparePipesByIdentity(Set<EdgeIdentity> leftEdges, Set<EdgeIdentity> rightEdges) {
        Map<String, EdgeIdentity> leftById = leftEdges.stream()
                .collect(Collectors.toMap(EdgeIdentity::id, edge -> edge));

        Map<String, EdgeIdentity> rightById = rightEdges.stream()
                .collect(Collectors.toMap(EdgeIdentity::id, edge -> edge));

        List<String> matched = new ArrayList<>();
        List<String> leftOnly = new ArrayList<>();
        List<String> rightOnly = new ArrayList<>();
        List<String> contextChanged = new ArrayList<>();

        for (var entry : leftById.entrySet()) {
            String id = entry.getKey();
            EdgeIdentity left = entry.getValue();
            EdgeIdentity right = rightById.get(id);
            if (right == null) {
                leftOnly.add(id);
                continue;
            }

            if (Objects.equals(left.from(), right.from()) && Objects.equals(left.to(), right.to())) {
                matched.add(id);
            } else {
                contextChanged.add(id);
                leftOnly.add(id);
                rightOnly.add(id);
            }
        }

        for (String id : rightById.keySet()) {
            if (!leftById.containsKey(id)) {
                rightOnly.add(id);
            }
        }

        matched.sort(String::compareTo);
        leftOnly.sort(String::compareTo);
        rightOnly.sort(String::compareTo);
        contextChanged.sort(String::compareTo);

        return new ComparableEntitySummary(
                List.copyOf(matched),
                List.copyOf(leftOnly),
                List.copyOf(rightOnly),
                List.copyOf(contextChanged)
        );
    }
}