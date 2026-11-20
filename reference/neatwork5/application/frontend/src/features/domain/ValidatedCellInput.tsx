import { useEffect, useState } from "react";
import { formatEditableTableCell } from "./tableUtils";
import { parseDomainEditorCell } from "./domainEditorValidation";
import type { DisplayLengthUnit } from "./unitSystem";

type Props = {
    value: unknown;
    column: string;
    displayUnit: DisplayLengthUnit;
    onCommit: (value: unknown) => void;
    onInvalid?: (message: string) => void;
};


export function ValidatedCellInput({ value, column, displayUnit, onCommit, onInvalid }: Props) {

    const formatted = formatEditableTableCell(value, column, displayUnit);


    const [rawValue, setRawValue] = useState(formatted);
    const [error, setError] = useState<string | null>(null);


    useEffect(() => {
        setRawValue(formatted);
        setError(null);
    }, [formatted]);


    function commit() {
        const result = parseDomainEditorCell(rawValue, column, displayUnit, value);

        if (!result.ok) {
            setError(result.message);
            onInvalid?.(result.message);
            return;
        }

        setError(null);
        onCommit(result.value);
        setRawValue(formatEditableTableCell(result.value, column, displayUnit));
    }

    return (
        <input type="text" value={rawValue}
            aria-invalid={error ? "true" : undefined}
            title={error ?? undefined}
            className={error ? "domain-cell-input invalid" : "domain-cell-input"}
            onChange={event => setRawValue(event.target.value)}
            onBlur={commit}
            onKeyDown={
                event => {
                    if (event.key === "Enter") {
                        event.preventDefault();
                        commit();
                    }
                    if (event.key === "Escape") {
                        setRawValue(formatted);
                        setError(null);
                    }
                }} />
    );
}