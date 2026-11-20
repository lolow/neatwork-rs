import { useState } from "react";
import type { FormEvent } from "react";
import type { NetworkNode } from "./types";


type Props = {
    nodes: NetworkNode[];
    disabled?: boolean;
    onLocate: (nodeId: string) => void;
};

/**
 * Search nodes by ID or label and request visual focus.
 */
export function NetworkNodeSearch({ nodes, disabled = false, onLocate, }: Props) {

    const [query, setQuery] = useState("");
    const [message, setMessage] = useState<string | null>(null);
    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const normalized = query.trim().toLowerCase();

        if (!normalized) {
            setMessage("Enter a node ID.");
            return;
        }

        /*
         * Exact match has priority.
         */
        const exactMatch =
            nodes.find(node => node.id.toLowerCase() === normalized
                || node.label.toLowerCase() === normalized);

        if (exactMatch) {
            setMessage(null);
            onLocate(exactMatch.id);
            return;
        }

        /*
         * Also allow one unique partial match.
         */
        const partialMatches = nodes.filter(
            node => node.id.toLowerCase().includes(normalized)
                || node.label.toLowerCase().includes(normalized)
        );

        if (partialMatches.length === 1) {
            setMessage(null);
            onLocate(partialMatches[0].id);
            return;
        }

        if (partialMatches.length > 1) {
            setMessage(`${partialMatches.length} nodes match. Enter a more specific ID.`);
            return;
        }
        setMessage("Node not found.");
    }

    return (
        <div className="network-node-search-wrap">
            <form className="network-node-search" onSubmit={handleSubmit}>

                <input value={query} disabled={disabled} placeholder="Search node..." aria-label="Search node"
                    onChange={event => {
                        setQuery(event.target.value);
                        setMessage(null);
                    }
                    }
                />
                <button type="submit" disabled={disabled || !query.trim()} >
                    Find
                </button>

            </form>

            {
                message && (
                    <span className="network-node-search-message" role="status">
                        {message}
                    </span>
                )
            }

        </div>
    );
}