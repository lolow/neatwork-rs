export type NodeSemanticType =
    | "SOURCE"
    | "JUNCTION"
    | "TERMINAL";


export function nodeSemanticLabel(
    type:
        NodeSemanticType
): string {

    switch (type) {

        case "SOURCE":
            return "Source";

        case "JUNCTION":
            return "Junction";

        case "TERMINAL":
            return "Terminal";
    }
}


export function nodeSemanticClassName(
    type:
        NodeSemanticType
): string {

    return type
        .toLowerCase();
}


export function nodeSemanticFromStatus(
    value:
        unknown
): NodeSemanticType | null {

    if (
        value === 0
        ||
        value === "0"
    ) {

        return "SOURCE";
    }


    if (
        value === 1
        ||
        value === "1"
    ) {

        return "JUNCTION";
    }


    if (
        value === 2
        ||
        value === "2"
    ) {

        return "TERMINAL";
    }


    const normalized =
        String(
            value ?? ""
        )
            .trim()
            .toUpperCase();


    if (
        normalized === "SOURCE"
    ) {

        return "SOURCE";
    }


    if (
        normalized === "JUNCTION"
        ||
        normalized === "TRANSIT"
    ) {

        return "JUNCTION";
    }


    if (
        normalized === "TERMINAL"
    ) {

        return "TERMINAL";
    }


    return null;
}

export function NodeTypeBadge({
    type,
}: {
    type:
    NodeSemanticType
    | null
    | undefined;
}) {

    if (!type) {

        return (
            <span
                className=
                "node-semantic-empty"
            >
                —
            </span>
        );
    }


    return (
        <span
            className={
                `node-semantic-badge `
                +
                nodeSemanticClassName(
                    type
                )
            }
        >

            <span
                className=
                "node-semantic-dot"
            />

            {
                nodeSemanticLabel(
                    type
                )
            }

        </span>
    );
}

export function NodeIdBadge({
    nodeId,
    type,
}: {
    nodeId:
    unknown;

    type:
    NodeSemanticType
    | null
    | undefined;
}) {

    const text =
        String(
            nodeId ?? ""
        );


    if (!text) {

        return (
            <span>
                —
            </span>
        );
    }


    if (!type) {

        return (
            <span>
                {text}
            </span>
        );
    }


    return (
        <span
            className={
                `node-semantic-id `
                +
                nodeSemanticClassName(
                    type
                )
            }
        >
            {text}
        </span>
    );
}