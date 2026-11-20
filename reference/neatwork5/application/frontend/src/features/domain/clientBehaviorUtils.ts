export type ResolvedClientDemand = {
    clientId: string;
    terminalId: string;
    behaviorId: string | null;
    openProbability: number;
    targetFlowLps: number;
};

type DomainDraft = {
    tables?: Record<string, unknown[]>;
    [key: string]: unknown;
};

export function resolveClientDemands(tables: Record<string, unknown>): ResolvedClientDemand[] {
    const behaviorRows = Array.isArray(tables.clientBehaviors)
        ? tables.clientBehaviors
        : Array.isArray(tables.client_behaviors)
            ? tables.client_behaviors
            : [];

    const clients = Array.isArray(tables.clients)
        ? tables.clients
        : [];

    const behaviorById = new Map<string, { probability: number; target: number; }>();

    for (const raw of behaviorRows) {
        if (!raw || typeof raw !== "object") {
            continue;
        }

        const row = raw as Record<string, unknown>;
        const behaviorId = String(row.behaviorId ?? row.behavior_id ?? "");
        const probability = Number(row.openProbability ?? row.open_probability);
        const target = Number(row.targetFlowLps ?? row.target_flow_lps);

        if (!behaviorId || !Number.isFinite(probability) || !Number.isFinite(target)) {
            continue;
        }
        behaviorById.set(behaviorId, { probability, target });
    }

    const resolved: ResolvedClientDemand[] = [];
    for (const raw of clients) {
        if (!raw || typeof raw !== "object") {
            continue;
        }

        const row = raw as Record<string, unknown>;
        const clientId = String(row.client ?? row.clientId ?? "");
        const terminalId = String(row.borne ?? row.terminalId ?? "");
        const behaviorId = String(row.behaviorId ?? row.behavior_id ?? "") || null;

        if (behaviorId) {
            const behavior = behaviorById.get(behaviorId);
            if (!behavior) {
                continue;
            }

            resolved.push({
                clientId,
                terminalId,
                behaviorId,
                openProbability: behavior.probability,
                targetFlowLps: behavior.target,
            });

            continue;
        }

        const probability = Number(row.proba ?? row.openProbability);
        const target = Number(row.cible ?? row.targetFlowLps);

        if (!Number.isFinite(probability) || !Number.isFinite(target)) {
            continue;
        }

        resolved.push({
            clientId,
            terminalId,
            behaviorId: null,
            openProbability: probability,
            targetFlowLps: target,
        });
    }
    return resolved;
}

export function normalizeClientBehaviorDraft<T extends DomainDraft>(domain: T): T {
    const tables = domain.tables ?? {};
    const existingBehaviors = Array.isArray(tables.clientBehaviors)
        ? tables.clientBehaviors
        : Array.isArray(tables.client_behaviors)
            ? tables.client_behaviors
            : [];

    const clients = Array.isArray(tables.clients) ? tables.clients : [];

    const allClientsCanonical = clients.every(raw => {
        if (!raw || typeof raw !== "object") {
            return true;
        }
        const row = raw as Record<string, unknown>;
        return Boolean(row.behaviorId ?? row.behavior_id);
    });


    if (allClientsCanonical) {
        return domain;
    }

    const behaviorByKey = new Map<string, Record<string, unknown>>();
    for (const raw of existingBehaviors) {
        if (!raw || typeof raw !== "object") {
            continue;
        }
        const behavior = raw as Record<string, unknown>;
        const probability = Number(behavior.openProbability ?? behavior.open_probability);
        const target = Number(behavior.targetFlowLps ?? behavior.target_flow_lps);
        const profile = String(behavior.profileId ?? behavior.profile_id ?? "");

        if (!Number.isFinite(probability) || !Number.isFinite(target)) {
            continue;
        }
        const key = `${probability}|${target}|${profile}`;
        behaviorByKey.set(key, behavior);
    }
    let sequence = 0;

    const normalizedClients = clients.map(
        raw => {
            if (!raw || typeof raw !== "object") {
                return raw;
            }
            const row = raw as Record<string, unknown>;
            const probability = Number(row.proba ?? row.openProbability);
            const target = Number(row.cible ?? row.targetFlowLps);
            if (!Number.isFinite(probability) || !Number.isFinite(target)) {
                return row;
            }
            const key = `${probability}|${target}|`;
            let behavior = behaviorByKey.get(key);
            if (!behavior) {
                sequence += 1;
                const behaviorId = `CB_LEGACY_${String(sequence).padStart(3, "0")}`;
                behavior = {
                    behaviorId,
                    name: `Imported behavior ${sequence}`,
                    openProbability: probability,
                    targetFlowLps: target,
                    profileId: null,
                };
                behaviorByKey.set(key, behavior);
            }
            return {
                client: row.client ?? row.clientId,
                borne: row.borne ?? row.terminalId,
                behaviorId: behavior.behaviorId,
            };
        }
    );

    return {
        ...domain,

        tables: {
            ...tables,
            clients: normalizedClients,
            clientBehaviors: [
                ...existingBehaviors,
                ...behaviorByKey.values(),
            ],
        },
    };
}