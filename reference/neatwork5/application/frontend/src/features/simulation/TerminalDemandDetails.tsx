import type { TerminalDemandProfile } from "./types";
import { formatAdaptiveNumber, formatRatioAsPercent } from "./resultNumberFormatting";

type Props = {
    profile: TerminalDemandProfile | undefined;
};

function behaviorLabel(names: string[], ids: string[]): string {

    if (names.length > 0) {
        return names.join(" / ");
    }

    if (ids.length > 0) {
        return ids.join(" / ");
    }
    return "Legacy behavior";
}

export function TerminalDemandDetails({ profile }: Props) {

    if (!profile) {
        return (
            <div className="terminal-demand-details terminal-demand-details--empty">
                Demand details are unavailable for this historical simulation.
            </div>
        );
    }

    if (profile.groups.length === 0) {
        return (
            <div className="terminal-demand-details terminal-demand-details--empty">
                No client is attached to this terminal.
            </div>
        );
    }

    /* return (
        <div className="terminal-demand-details">

            <div className="terminal-demand-details__heading">
                Demand groups
            </div>
            <div className="terminal-demand-details__groups">
                {profile.groups.map(
                    group => (
                        <div key={group.groupId} className="terminal-demand-group">
                            <strong>
                                {
                                    behaviorLabel(group.behaviorNames, group.behaviorIds)
                                }
                            </strong>

                            <span>
                                {group.clientCount} {" "}
                                {group.clientCount === 1 ? "client" : "clients"}
                            </span>
                            <span>
                                Target flow: {" "}
                                <strong>
                                    {formatAdaptiveNumber(group.targetFlowLps)} {" L/s"}
                                </strong>
                                {" / client"}
                            </span>

                            <span>
                                Open probability:{" "}
                                <strong>
                                    {formatRatioAsPercent(group.openProbability)}
                                </strong>
                            </span>
                        </div>
                    ))}
            </div>
        </div>
    ); */
}