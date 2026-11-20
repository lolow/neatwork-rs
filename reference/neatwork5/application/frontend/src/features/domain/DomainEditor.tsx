import { useMemo, useState, useEffect } from "react";
import type {
    ClipboardEvent as ReactClipboardEvent,
    KeyboardEvent as ReactKeyboardEvent,
    MouseEvent as ReactMouseEvent,
} from "react";
import type { ProjectArtifact } from "../artifacts/types";
import type { DomainSnapshot, RevisionNamingMode } from "./types";
import {
    asDomainPayload, buildDomainTableViews, getTableColumns,
    formatEditableTableCell, labelTableColumn
} from "./tableUtils";
import { SaveRevisionModeModal } from "./SaveRevisionModeModal";
import { CustomRevisionNameModal } from "./CustomRevisionNameModal";
import type { DisplayLengthUnit } from "./unitSystem";
import { checkCatalogDeletion } from "./catalogIntegrity";
import type { CatalogDeletionCheck } from "./catalogIntegrity";
import { findBehaviorReferences, findDuplicateBehaviorIds } from "./clientBehaviorIntegrity";
import { normalizeClientBehaviorDraft } from "./clientBehaviorUtils";
import { ValidatedCellInput } from "./ValidatedCellInput";
import { parseDomainEditorCell, validateStoredNumericValue } from "./domainEditorValidation";


type Props = {
    snapshot: DomainSnapshot;
    sourceArtifact: ProjectArtifact;
    displayUnit: DisplayLengthUnit;
    saving: boolean;
    onSave: (domain: unknown, namingMode: RevisionNamingMode, customFilename?: string | null) => Promise<void>;
    onCancel: () => void;
};

function cloneDomain(value: unknown) {
    return typeof structuredClone === "function"
        ? structuredClone(value)
        : JSON.parse(JSON.stringify(value));
}


export function DomainEditor({
    snapshot,
    sourceArtifact,
    displayUnit,
    saving,
    onSave,
    onCancel,
}: Props) {
    const [draft, setDraft] = useState(() => {
        const initial = asDomainPayload(cloneDomain(snapshot.domain));
        return normalizeClientBehaviorDraft(initial);
    });
    const [selectedTableKey, setSelectedTableKey] = useState<string | null>(null);
    const [editorSection, setEditorSection] = useState<"tables" | "parameters">("tables");
    const [editorValidationError, setEditorValidationError] = useState<string | null>(null);
    const [saveModeOpen, setSaveModeOpen] = useState(false);
    const [customNameOpen, setCustomNameOpen] = useState(false);
    const tableViews = useMemo(() => buildDomainTableViews(draft.tables ?? {}), [draft.tables]);
    const selectedTable = tableViews.find(table => table.key === selectedTableKey) ?? tableViews[0];
    const clientBehaviors = useMemo(() => {
        const tables = draft.tables ?? {};
        const rows = tables.clientBehaviors ?? tables.client_behaviors ?? [];
        return rows.filter(row => row && typeof row === "object").map(
            raw => {
                const row = raw as Record<string, unknown>;
                return {
                    id: String(row.behaviorId ?? row.behavior_id ?? ""),
                    name: String(row.name ?? ""),
                };
            }
        ).filter(behavior => behavior.id);
    }, [draft.tables]);
    const [blockedDeletion, setBlockedDeletion] = useState<CatalogDeletionCheck | null>(null);

    // Updates one table cell without mutating the immutable snapshot received from App.
    function updateCell(tableKey: string, rowIndex: number, column: string, value: unknown) {
        setDraft(current => {
            const tables = { ...(current.tables ?? {}) };
            const rows = [...(tables[tableKey] ?? [])];
            const previous = rows[rowIndex] as Record<string, unknown>;
            rows[rowIndex] = { ...previous, [column]: value };
            tables[tableKey] = rows;

            return { ...current, tables };
        });
        setEditorValidationError(null);
    }

    // Updates the same column across multiple selected rows in one immutable draft operation.
    // Each value still passes through the normal unit/type conversion logic before being stored.
    function updateCells(tableKey: string, rowIndexes: number[], column: string, rawValue: string): string | null {
        const rows = draft.tables?.[tableKey] ?? [];
        const parsed: { rowIndex: number; value: unknown; }[] = [];
        for (const rowIndex of rowIndexes) {
            const row = rows[rowIndex] as Record<string, unknown> | undefined;
            if (!row) {
                continue;
            }
            const result = parseDomainEditorCell(rawValue, column, displayUnit, row[column]);
            if (!result.ok) {
                return (`Row ${rowIndex + 1}, ` +
                    `${labelTableColumn(column, displayUnit)}: `
                    +
                    result.message
                );
            }
            parsed.push({ rowIndex, value: result.value });
        }

        setDraft(current => {
            const tables = { ...(current.tables ?? {}) };
            const nextRows = [...(tables[tableKey] ?? [])];
            for (const update of parsed) {
                const previous = nextRows[update.rowIndex] as Record<string, unknown>;
                nextRows[update.rowIndex] = {
                    ...previous,
                    [column]: update.value,
                };
            }
            tables[tableKey] = nextRows;
            return { ...current, tables };
        });
        return null;
    }

    // Applies a rectangular clipboard update in one immutable draft operation.
    // Every pasted display value goes through the same parser and unit-conversion path as ordinary editing.
    function updateCellBatch(tableKey: string, updates: CellBatchUpdate[]): string | null {

        if (updates.length === 0) {
            return null;
        }
        const rows = draft.tables?.[tableKey] ?? [];
        const parsed: { rowIndex: number; column: string; value: unknown; }[] = [];
        for (const update of updates) {
            const row = rows[update.rowIndex] as Record<string, unknown> | undefined;
            if (!row) {
                continue;
            }
            const result = parseDomainEditorCell(
                update.value, update.column,
                displayUnit, row[update.column]
            );

            if (!result.ok) {
                return (`Invalid pasted value at row ${update.rowIndex + 1}, `
                    +
                    `${labelTableColumn(update.column, displayUnit)}: `
                    +
                    result.message
                );
            }

            parsed.push(
                {
                    rowIndex: update.rowIndex,
                    column: update.column,
                    value: result.value,
                }
            );
        }

        setDraft(current => {
            const tables = { ...(current.tables ?? {}) };
            const nextRows = [...(tables[tableKey] ?? [])];
            for (const update of parsed) {

                const previous = nextRows[update.rowIndex] as Record<string, unknown>;
                nextRows[update.rowIndex] = {
                    ...previous,
                    [update.column]: update.value,
                };
            }
            tables[tableKey] = nextRows;
            return { ...current, tables };
        }
        );
        return null;

    }

    // Seed belongs to a simulation execution, not to the persistent hydraulic project configuration.
    function isSimulationSeedParameter(key: string): boolean {
        const normalized = key.trim().toLowerCase().replace(/[-_\s]/g, "");
        return (normalized === "seed" || normalized === "randomseed");
    }

    // Adds a blank row using the currently known table schema.
    function addRow(tableKey: string, columns: string[]) {
        setDraft(current => {
            const tables = { ...(current.tables ?? {}) };
            const rows = [...(tables[tableKey] ?? [])];
            rows.push(Object.fromEntries(columns.map(column => [column, ""])));

            return {
                ...current,
                tables: {
                    ...tables,
                    [tableKey]: rows,
                },
            };
        });
    }

    // Validates that all client behavior IDs in the editable draft are unique.
    // If duplicates are found, sets an error message and returns false.
    // If no duplicates are found, clears any existing error and returns true.
    function validateBehaviorIds(): boolean {
        const tables = draft.tables ?? {};
        const rawBehaviors = tables.clientBehaviors ?? tables.client_behaviors ?? [];
        const behaviors = rawBehaviors.filter(row => row && typeof row === "object") as Record<
            string,
            unknown
        >[];
        const duplicates = findDuplicateBehaviorIds(behaviors);

        if (duplicates.length === 0) {
            setEditorValidationError(null);
            return true;
        }
        setEditorValidationError(`Duplicate client behavior ID: ${duplicates.join(", ")}`);
        return false;
    }

    async function saveDraft(namingMode: RevisionNamingMode, customFilename?: string | null): Promise<boolean> {
        setEditorValidationError(null);
        if (!validateBehaviorIds() || !validateNumericTables()) {
            return false;
        }
        try {
            await onSave(draft, namingMode, customFilename ?? null);
            return true;
        } catch (error) {
            console.error("Revision save failed:", error);
            setEditorValidationError(error instanceof Error
                ? error.message
                : "The edited revision could not be saved."
            );
            return false;
        }
    }

    // Removes one row only from the editable draft.
    // The persisted source file remains untouched until a new revision is explicitly saved.
    function deleteRow(tableKey: string, rowIndex: number) {
        const normalizedTableKey = tableKey.trim().toLowerCase().replace(/[\s_-]+/g, "");
        if (normalizedTableKey === "clientbehaviors") {
            const behaviorRows = draft.tables?.[tableKey] ?? [];
            const row = behaviorRows[rowIndex] as Record<string, unknown> | undefined;
            const behaviorId = String(row?.behaviorId ?? row?.behavior_id ?? "");
            const clients = findClientRows(draft.tables ?? {});
            const references = findBehaviorReferences(behaviorId, clients);
            if (references.length > 0) {
                setBlockedDeletion({
                    blocked: true,
                    catalogType: null,
                    itemLabel: behaviorId,
                    references:
                        references.map(
                            clientId => ({
                                entityId: clientId,
                                location: "Client behavior reference",
                            })),
                });

                return;
            }
        }
        const check = checkCatalogDeletion(draft, tableKey, rowIndex);
        if (check.blocked) {
            setBlockedDeletion(check);
            return;
        }
        setDraft(current => {
            const tables = { ...(current.tables ?? {}) };
            const rows = [...(tables[tableKey] ?? [])];
            rows.splice(rowIndex, 1);
            tables[tableKey] = rows;
            return { ...current, tables };
        });
    }

    // Update one project default parameter while converting display-unit input back to canonical data.
    function updateParameter(key: string, value: unknown) {
        setDraft(current => ({
            ...current,
            params: {
                ...(current.params ?? {}),
                [key]: value,
            },
        }));
    }

    function validateNumericTables(): boolean {
        for (const table of tableViews) {
            const columns = getTableColumns(table.rows);

            for (let rowIndex = 0; rowIndex < table.rows.length; rowIndex += 1) {
                const row = table.rows[rowIndex];
                for (const column of columns) {
                    const error = validateStoredNumericValue(column, row[column]);
                    if (error) {
                        setEditorValidationError(
                            `${table.title}, row ` + `${rowIndex + 1}, ` +
                            `${labelTableColumn(column, displayUnit)}: ` + error
                        );
                        return false;
                    }
                }
            }
        }
        return true;
    }

    return (
        <section className="domain-editor">
            <header className="domain-editor__header">
                <div>
                    <span>Editing input data</span>
                    <p>Changes are stored only when a new revision is created.</p>
                </div>


                <div className="domain-editor__header-actions">
                    <button type="button" className="project-modal__secondary"
                        disabled={saving}
                        onClick={onCancel}
                    >
                        Cancel
                    </button>

                    <button type="button" className="project-modal__primary"
                        disabled={saving}
                        onClick={() => setSaveModeOpen(true)}
                    >
                        Save
                    </button>
                </div>
            </header>

            <div className="domain-editor__mode-tabs">
                <button type="button"
                    className={editorSection === "tables" ? "active" : ""}
                    onClick={() => setEditorSection("tables")}
                >
                    Input tables
                </button>

                <button type="button"
                    className={editorSection === "parameters" ? "active" : ""}
                    onClick={() => setEditorSection("parameters")}
                >
                    Default properties
                </button>
            </div>

            {editorSection === "tables" && (
                <div className="neatwork-table-layout">
                    <aside className="neatwork-table-tabs">
                        {tableViews.map(table => (
                            <button
                                key={table.key}
                                type="button"
                                className={selectedTable?.key === table.key ? "active" : ""}
                                onClick={() => setSelectedTableKey(table.key)}
                            >
                                <span>{table.title}</span>
                                <small>{table.rows.length}{" rows"}</small>
                            </button>
                        ))}
                    </aside>

                    {selectedTable && (
                        <EditableTable
                            table={selectedTable}
                            clientBehaviors={clientBehaviors}
                            onChange={updateCell}
                            onBulkChange={updateCells}
                            onBatchChange={updateCellBatch}
                            onAddRow={addRow}
                            onDeleteRow={deleteRow}
                            displayUnit={displayUnit}
                        />
                    )}
                </div>
            )}

            {editorValidationError && (
                <div className="domain-editor-validation-error">
                    {editorValidationError}
                </div>
            )}

            {editorSection === "parameters" && (
                <div className="domain-parameter-editor">
                    {Object.entries(draft.params ?? {}).filter(
                        ([key]) => !isSimulationSeedParameter(key))
                        .map(([key, value]) => (
                            <label key={key} className="domain-parameter-field">
                                <span>{key}</span>
                                <ValidatedCellInput
                                    value={value}
                                    column={key}
                                    displayUnit={displayUnit}
                                    onCommit={normalized => updateParameter(key, normalized)}
                                    onInvalid={message => setEditorValidationError(`${key}: ${message}`)}
                                />
                            </label>
                        )
                        )}

                </div>
            )}


            {saveModeOpen && (
                <SaveRevisionModeModal
                    sourceArtifact={sourceArtifact}
                    saving={saving}
                    error={editorValidationError}
                    onAutomaticSave={() => {
                        void (
                            async () => {
                                const saved = await saveDraft("DEFAULT", null);
                                if (saved) {
                                    setSaveModeOpen(false);
                                }
                            }
                        )();
                    }}
                    onCustomSave={() => {
                        setSaveModeOpen(false);
                        setCustomNameOpen(true);
                    }}

                    onCancel={() => setSaveModeOpen(false)}
                />
            )}


            {customNameOpen && (
                <CustomRevisionNameModal
                    sourceArtifact={sourceArtifact}
                    saving={saving}
                    onBack={() => {
                        setCustomNameOpen(false);
                        setSaveModeOpen(true);
                    }}
                    onSave={filename => {
                        void (
                            async () => {
                                const saved = await saveDraft("CUSTOM", filename);
                                if (saved) {
                                    setCustomNameOpen(false);
                                }
                            }
                        )();
                    }}
                    onCancel={() => setCustomNameOpen(false)}
                />
            )}

            {blockedDeletion && (<div className="project-modal-backdrop">
                <section className="project-modal catalog-delete-blocked-modal"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="catalog-delete-blocked-title">

                    <header className="project-modal__header">
                        <div>
                            <span className="project-modal__eyebrow">Catalog item in use</span>
                            <h2 id="catalog-delete-blocked-title">
                                This item cannot be deleted
                            </h2>
                            <p>
                                {blockedDeletion.catalogType === "NODE"
                                    ? "This node is still referenced by the network." :
                                    blockedDeletion.catalogType === "PIPE_DIAMETER"
                                        ? "This pipe diameter is still referenced by the network."
                                        : blockedDeletion.catalogType === "ORIFICE_DIAMETER"
                                            ? ("This orifice diameter is still referenced by terminal nodes.")
                                            : ("This client behavior is still referenced by one or more clients.")
                                }
                            </p>
                        </div>
                    </header>
                    <div className="project-modal__body">
                        <div className="catalog-delete-blocked-item">
                            <span>{blockedDeletion.catalogType === null ? "Client behavior" : "Catalog item"}</span>
                            <strong>
                                {blockedDeletion.itemLabel}
                            </strong>
                        </div>

                        <div className="catalog-delete-reference-summary">

                            <strong>
                                {blockedDeletion.references.length}
                                {" reference"}
                                {blockedDeletion.references.length === 1
                                    ? ""
                                    : "s"}
                            </strong>
                            <span>
                                Replace these references before deleting the catalog item.
                            </span>
                        </div>
                        <div className="catalog-delete-reference-list">
                            {blockedDeletion.references
                                .slice(0, 20)
                                .map((reference, index) => (
                                    <div key={`${reference.entityId}:${reference.location}:${index}`}>
                                        <strong>
                                            {reference.entityId}
                                        </strong>
                                        <span>
                                            {reference.location}
                                        </span>
                                    </div>
                                ))}
                        </div>

                        {blockedDeletion.references.length > 20 && (
                            <small>
                                +
                                {blockedDeletion.references.length - 20}
                                {" more references"}
                            </small>
                        )}
                    </div>


                    <footer className="project-modal__footer">
                        <button type="button"
                            className="project-modal__primary"
                            onClick={() => setBlockedDeletion(null)}>
                            OK
                        </button>
                    </footer>
                </section>
            </div>
            )}
        </section>
    );
}

type CellBatchUpdate = {
    rowIndex: number;
    column: string;
    value: string;
};

type EditableTableProps = {
    table: {
        key: string;
        title: string;
        rows: Record<string, unknown>[];
    };

    onChange: (
        tableKey: string,
        rowIndex: number,
        column: string,
        value: unknown
    ) => void;

    onBulkChange: (
        tableKey: string,
        rowIndexes: number[],
        column: string,
        value: string
    ) => string | null;

    onBatchChange: (
        tableKey: string,
        updates: CellBatchUpdate[]
    ) => string | null;

    onAddRow: (
        tableKey: string,
        columns: string[]
    ) => void;

    onDeleteRow: (
        tableKey: string,
        rowIndex: number
    ) => void;

    clientBehaviors: {
        id: string;
        name: string;
    }[];

    displayUnit: DisplayLengthUnit;
};

type CellRangeSelection = {
    anchorRow: number;
    focusRow: number;
    anchorColumn: number;
    focusColumn: number;
};

// Normalize the column name by removing whitespace and converting to lowercase.
function normalizeEditorColumn(column: string): string {
    return column.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

// Detects editable node-status fields that use the canonical NeatWork values 0, 1 and 2.
function isEditableStatusColumn(column: string): boolean {
    return [
        "statut",
        "status",
        "statuscode",
        "nodestatus",
        "nodestatuscode",
    ].includes(normalizeEditorColumn(column));
}

// Parses the standard tab/newline clipboard format produced by Excel, LibreOffice and spreadsheet apps.
function parseClipboardMatrix(text: string): string[][] {
    const normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    const rows = normalized.split("\n");
    // Spreadsheet applications commonly append one newline after the copied range.
    if (rows.length > 1 && rows[rows.length - 1] === "") {
        rows.pop();
    }
    return rows.map(row => row.split("\t"));
}

function clipboardMatrixWidth(matrix: string[][]): number {
    return matrix.reduce(
        (maximum, row) => Math.max(maximum, row.length), 0
    );
}

function findClientRows(tables: Record<string, unknown[]>): Record<string, unknown>[] {
    const aliases = new Set(["clients", "clientlist", "generatedclients"]);
    for (const [key, rows] of Object.entries(tables)) {
        const normalized = key.trim().toLowerCase().replace(/[\s_-]+/g, "");
        if (aliases.has(normalized) && Array.isArray(rows)) {
            return rows as Record<string, unknown>[];
        }
    }

    return [];
}

function EditableTable({ table, clientBehaviors, onChange, onBulkChange, onBatchChange, onAddRow, onDeleteRow, displayUnit }: EditableTableProps) {

    const [selection, setSelection] = useState<CellRangeSelection | null>(null);
    const [dragging, setDragging] = useState(false);
    const [bulkValue, setBulkValue] = useState("");
    const [clipboardMessage, setClipboardMessage] = useState<string | null>(null);
    const [clipboardError, setClipboardError] = useState<string | null>(null);

    // Calculate the selection start and end indexes based on the selection state.
    const selectionRowStart = selection ? Math.min(selection.anchorRow, selection.focusRow) : null;
    const selectionRowEnd = selection ? Math.max(selection.anchorRow, selection.focusRow) : null;
    const selectionColumnStart = selection ? Math.min(selection.anchorColumn, selection.focusColumn) : null;
    const selectionColumnEnd = selection ? Math.max(selection.anchorColumn, selection.focusColumn) : null;
    // Get the column headers from the first row of the table.
    const columns = getTableColumns(table.rows);

    // Reset the selection and bulk value when the table changes.
    useEffect(() => {
        setSelection(null);
        setDragging(false);
        setBulkValue("");
        setClipboardMessage(null);
        setClipboardError(null);
    }, [table.key]);

    // Stop dragging when the mouse button is released.
    useEffect(() => {
        function stopDragging() {
            setDragging(false);
        }
        window.addEventListener("mouseup", stopDragging);
        return () => {
            window.removeEventListener("mouseup", stopDragging);
        };
    }, []);

    // Calculate the selected row indexes based on the selection state.
    const selectedRowIndexes = useMemo(() => {
        if (selectionRowStart === null || selectionRowEnd === null) {
            return [];
        }
        return Array.from(
            { length: selectionRowEnd - selectionRowStart + 1, },
            (_, offset) => selectionRowStart + offset
        );
    }, [selectionRowStart, selectionRowEnd]);

    const selectedColumnIndexes = useMemo(() => {
        if (selectionColumnStart === null || selectionColumnEnd === null) {
            return [];
        }
        return Array.from(
            { length: selectionColumnEnd - selectionColumnStart + 1 },
            (_, offset) => selectionColumnStart + offset);
    }, [selectionColumnStart, selectionColumnEnd]);

    // Map the column indexes to the actual column headers.
    const selectedColumns = selectedColumnIndexes
        .map(index => columns[index])
        .filter((column): column is string => Boolean(column));

    // Calculate the total number of selected cells.
    const selectedCellCount = selectedRowIndexes.length * selectedColumns.length;

    //  function to determine if a cell is selected based on the selection state.
    function isSelectedCell(rowIndex: number, columnIndex: number): boolean {
        if (selectionRowStart === null || selectionRowEnd === null ||
            selectionColumnStart === null || selectionColumnEnd === null) {
            return false;
        }

        return (
            rowIndex >= selectionRowStart &&
            rowIndex <= selectionRowEnd &&
            columnIndex >= selectionColumnStart &&
            columnIndex <= selectionColumnEnd
        );
    }

    // Starts a new vertical selection or extends the existing range when Shift is held.
    function handleCellMouseDown(rowIndex: number, columnIndex: number, event: ReactMouseEvent<HTMLTableCellElement>) {
        if (event.button !== 0) {
            return;
        }
        setClipboardError(null);
        setClipboardMessage(null);

        // Shift extends the existing rectangular selection from its original anchor.
        if (event.shiftKey && selection) {
            setSelection(current => current ? {
                ...current,
                focusRow: rowIndex,
                focusColumn: columnIndex,
            } : null);
            setDragging(false);
            return;
        }

        setSelection({
            anchorRow: rowIndex,
            focusRow: rowIndex,
            anchorColumn: columnIndex,
            focusColumn: columnIndex,
        });
        setBulkValue("");
        setDragging(true);
    }

    // Extends a mouse selection only inside the same column where the drag originally started.
    function handleCellMouseEnter(rowIndex: number, columnIndex: number) {
        if (!dragging || !selection) {
            return;
        }
        setSelection(current => current ? { ...current, focusRow: rowIndex, focusColumn: columnIndex } : null);
    }

    // Clears the current cell selection and stops the dragging state.
    function clearSelection() {
        setSelection(null);
        setDragging(false);
        setBulkValue("");
        setClipboardMessage(null);
        setClipboardError(null);
    }

    // Applies one display-unit value to every selected cell. Canonical conversion remains the responsibility of DomainEditor's normal bulk update path.
    function applyBulkValue(value: string = bulkValue) {

        if (!selection || selectedRowIndexes.length < 2 || selectedColumns.length !== 1) {
            return;
        }
        const column = selectedColumns[0];
        if (!column) {
            return;
        }
        const error = onBulkChange(table.key, selectedRowIndexes, column, value);
        if (error) {
            setClipboardMessage(null);
            setClipboardError(error);
            return;
        }
        setClipboardError(null);
        setClipboardMessage(`Updated ${selectedRowIndexes.length} cells`);
        setBulkValue("");
    }

    // handles Enter and Escape key presses in the bulk value input.
    function handleBulkKeyDown(event: ReactKeyboardEvent<HTMLInputElement>) {
        if (event.key === "Enter") {
            event.preventDefault();
            if (bulkValue.trim() !== "") {
                applyBulkValue();
            }
        }
        if (event.key === "Escape") {
            event.preventDefault();
            clearSelection();
        }
    }

    // builds a tab-separated text block from the selected table cells.
    function buildSelectionClipboardText(): string | null {
        if (selectedRowIndexes.length === 0 || selectedColumns.length === 0) {
            return null;
        }
        return selectedRowIndexes.map(rowIndex => {
            const row = table.rows[rowIndex];
            if (!row) {
                return "";
            }
            return selectedColumns.map(column =>
                formatEditableTableCell(row[column], column, displayUnit))
                .join("\t");
        }).join("\n");
    }

    // handles the Copy keyboard shortcut and paste into the clipboard.
    function handleTableCopy(event: ReactClipboardEvent<HTMLDivElement>) {
        if (!selection || selectedCellCount === 0) {
            return;
        }

        // Preserve normal text copying when the user has explicitly selected text inside one input.
        const activeElement = document.activeElement;

        if (activeElement instanceof HTMLInputElement
            &&
            activeElement.selectionStart !== null
            &&
            activeElement.selectionEnd !== null
            &&
            activeElement.selectionStart !== activeElement.selectionEnd
        ) {
            return;
        }
        const text = buildSelectionClipboardText();
        if (text === null) {
            return;
        }
        event.preventDefault();
        event.clipboardData.setData("text/plain", text);
        setClipboardError(null);
        setClipboardMessage(`Copied ${selectedRowIndexes.length} × ${selectedColumns.length} cells`);
    }

    // builds a list of updates from the pasted matrix.
    function buildPasteUpdates(matrix: string[][]): CellBatchUpdate[] | null {
        if (selectionRowStart === null || selectionColumnStart === null) {
            return null;
        }
        const height = matrix.length;
        const width = clipboardMatrixWidth(matrix);
        if (height === 0 || width === 0) {
            return null;
        }
        // One clipboard value fills the whole selected range, matching common spreadsheet behaviour.
        if (height === 1 && width === 1 && selectedCellCount > 1) {
            const value = matrix[0]?.[0] ?? "";
            const updates: CellBatchUpdate[] = [];
            for (const rowIndex of selectedRowIndexes) {
                for (const column of selectedColumns) {
                    updates.push({ rowIndex, column, value });
                }
            }
            return updates;
        }

        const lastRow = selectionRowStart + height - 1;
        const lastColumn = selectionColumnStart + width - 1;

        if (lastRow >= table.rows.length || lastColumn >= columns.length) {
            return null;
        }

        const updates: CellBatchUpdate[] = [];

        matrix.forEach((clipboardRow, rowOffset) => {
            for (let columnOffset = 0; columnOffset < width; columnOffset += 1) {
                const column = columns[selectionColumnStart + columnOffset];
                if (!column) {
                    continue;
                }
                updates.push({
                    rowIndex: selectionRowStart + rowOffset,
                    column,
                    value: clipboardRow[columnOffset] ?? "",
                });
            }
        });

        return updates;
    }

    // handles the paste keyboard shortcut and pastes into the clipboard.
    function handleTablePaste(event: ReactClipboardEvent<HTMLDivElement>) {
        const text = event.clipboardData.getData("text/plain");
        if (text === "" || !selection) {
            return;
        }
        const matrix = parseClipboardMatrix(text);
        const height = matrix.length;
        const width = clipboardMatrixWidth(matrix);
        const isMultiCellPaste = height > 1 || width > 1;

        // For a single selected cell and a simple scalar paste, keep native input editing behaviour.
        if (selectedCellCount === 1 && !isMultiCellPaste && document.activeElement instanceof HTMLInputElement) {
            return;
        }
        const updates = buildPasteUpdates(matrix);
        if (!updates) {
            event.preventDefault();
            setClipboardMessage(null);
            setClipboardError("The pasted range does not fit inside this table.");
            return;
        }

        const validationError = onBatchChange(table.key, updates);
        if (validationError) {
            setClipboardMessage(null);
            setClipboardError(validationError);
            return;
        }

        event.preventDefault();
        setClipboardError(null);
        setClipboardMessage(updates.length === 1 ? "Pasted 1 cell" : `Pasted ${updates.length} cells`);
        // Select the newly pasted rectangle.
        if (height === 1 && width === 1 && selectedCellCount > 1) {
            return;
        }
        setSelection({
            anchorRow: selectionRowStart!,
            focusRow: selectionRowStart! + height - 1,
            anchorColumn: selectionColumnStart!,
            focusColumn: selectionColumnStart! + width - 1,
        });
    }

    return (
        <section className="neatwork-table-panel">
            <div className="neatwork-table-panel-header">
                <div>
                    <h3>{table.title}</h3>
                    <p>Edit input values.</p>
                </div>
                <div className="domain-editor-table-header-actions">
                    {clipboardMessage && (
                        <span className="domain-clipboard-message-inline">{clipboardMessage}</span>
                    )}
                    {clipboardError && (
                        <span className="domain-clipboard-error-inline">{clipboardError}</span>
                    )}
                    <div className="domain-clipboard-inline">
                        <div className="domain-clipboard-inline__selection">
                            {selection ? (
                                <>
                                    <strong>
                                        {selectedCellCount}
                                        {" cell"}
                                        {selectedCellCount === 1 ? "" : "s"}
                                        {" selected"}
                                    </strong>
                                    <span>
                                        {selectedRowIndexes.length}
                                        {" × "}
                                        {selectedColumns.length}
                                    </span>
                                </>
                            ) : (
                                <span> No selection</span>
                            )}
                        </div>
                        {/* start of the copy and paste shortcuts  */}
                        <div className="domain-clipboard-shortcuts">
                            <kbd> Ctrl/Cmd+C</kbd>
                            <span> Copy </span>
                            <kbd> Ctrl/Cmd+V</kbd>
                            <span> Paste </span>
                        </div>
                        <button type="button" className="project-modal__secondary" disabled={!selection} onClick={clearSelection} >
                            Clear
                        </button>
                    </div>
                    <button type="button" className="project-modal__secondary" onClick={() => {
                        clearSelection();
                        onAddRow(table.key, columns);
                    }}>
                        + Add row
                    </button>
                </div>
            </div>
            {/* Bulk editor appears only for a multi-row, single-column selection. */}
            {selection && selectedRowIndexes.length > 1 && selectedColumns.length === 1 && (
                <div className="domain-table-tools">
                    <div className="domain-bulk-editor">
                        <div className="domain-bulk-editor__summary">
                            <strong>{selectedRowIndexes.length}{" cells selected"}</strong>
                            <span>{selectedColumns[0] ? labelTableColumn(selectedColumns[0], displayUnit) : ""}</span>
                        </div>
                        {selectedColumns[0] && isEditableStatusColumn(selectedColumns[0]) && (
                            <div className="domain-bulk-editor__status-actions">
                                <span>Set status:</span>
                                <button type="button" className="source" onClick={() => applyBulkValue("0")}>Source</button>
                                <button type="button" className="junction" onClick={() => applyBulkValue("1")}>Junction</button>
                                <button type="button" className="terminal" onClick={() => applyBulkValue("2")}>Terminal</button>
                            </div>
                        )}
                        <div className="domain-bulk-editor__value">
                            <input type="text" value={bulkValue} placeholder="New value" onChange={event => setBulkValue(event.target.value)} onKeyDown={handleBulkKeyDown} />
                            <button type="button" className="project-modal__primary" disabled={bulkValue.trim() === ""} onClick={() => applyBulkValue()}>Apply to selected</button>
                            <button type="button" className="project-modal__secondary" onClick={clearSelection}>Clear selection</button>
                        </div>
                    </div>
                </div>
            )
            }

            <div className="domain-editor-clipboard-surface"
                onCopy={handleTableCopy}
                onPaste={handleTablePaste}
                onKeyDown={event => {
                    if (event.key === "Escape") {
                        clearSelection();
                    }
                }}>
                <div className="neatwork-table-scroll">
                    <table className={dragging
                        ? "editable-domain-table selecting"
                        : "editable-domain-table"
                    }>
                        <thead>
                            <tr>
                                <th>#</th>
                                {columns.map(
                                    column => (
                                        <th key={column}>
                                            {labelTableColumn(column, displayUnit)}
                                        </th>
                                    ))}
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {table.rows.map(
                                (row, rowIndex) => (
                                    <tr key={rowIndex}>
                                        <td>{rowIndex + 1}</td>
                                        {columns.map((column, columnIndex) => {
                                            const selected = isSelectedCell(rowIndex, columnIndex);
                                            const selectionTop = selected && rowIndex === selectionRowStart;
                                            const selectionBottom = selected && rowIndex === selectionRowEnd;
                                            const selectionLeft = selected && columnIndex === selectionColumnStart;
                                            const selectionRight = selected && columnIndex === selectionColumnEnd;
                                            const classes = ["editable-domain-cell",
                                                selected ? "selected" : "",
                                                selectionTop ? "selection-top" : "",
                                                selectionBottom ? "selection-bottom" : "",
                                                selectionLeft ? "selection-left" : "",
                                                selectionRight ? "selection-right" : "",
                                            ].filter(Boolean).join(" ");
                                            return (
                                                <td key={column}
                                                    className={classes}
                                                    onMouseDown={event => handleCellMouseDown(rowIndex, columnIndex, event)}
                                                    onMouseEnter={() => handleCellMouseEnter(rowIndex, columnIndex)}
                                                >
                                                    {(normalizeEditorColumn(table.key) === "clients" && (column === "behaviorId" || column === "behavior_id"))
                                                        ? (<select
                                                            value={String(row[column] ?? "")}
                                                            onChange={event => onChange(table.key, rowIndex, column, event.target.value)}
                                                        >
                                                            <option value="">Select behavior</option>
                                                            {
                                                                clientBehaviors.map(
                                                                    behavior => (
                                                                        <option key={behavior.id} value={behavior.id}>
                                                                            {behavior.name || behavior.id}
                                                                        </option>
                                                                    )
                                                                )
                                                            }
                                                        </select>
                                                        )
                                                        : (
                                                            <ValidatedCellInput
                                                                value={row[column]}
                                                                column={column}
                                                                displayUnit={displayUnit}
                                                                onCommit={value => onChange(table.key, rowIndex, column, value)}
                                                                onInvalid={message => setClipboardError(`${labelTableColumn(column, displayUnit)}: ${message}`)}
                                                            />
                                                        )
                                                    }
                                                </td>
                                            );
                                        })}

                                        <td>
                                            <button
                                                type="button"
                                                className="project-modal__danger-outline"
                                                onClick={() => { clearSelection(); onDeleteRow(table.key, rowIndex); }}>
                                                Delete
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                        </tbody>
                    </table>
                </div>
            </div>
        </section >
    );
}