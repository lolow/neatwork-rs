import {
    useEffect,
    useRef,
    useState,
    type ChangeEvent,
    type DragEvent,
    type KeyboardEvent as ReactKeyboardEvent
} from "react";
import { createPortal } from "react-dom";

export type NetworkInputFormat = "DSG" | "TPO";

type InputTypeModalProps = {
    projectName: string;
    onSelect: (format: NetworkInputFormat) => void;
    allowedFormat: NetworkInputFormat | null;
    blocked: boolean;

    //Indicates that a dropped/local file is currently being uploaded or imported.
    uploading: boolean;

    // Error message from the latest local file upload, or null if no error.
    error: string | null;

    //Handles a local DSG/TPO file without requiring the user to select its format first.
    onDropFile: (file: File) => Promise<void>;
    onClose: () => void;
};

export function InputTypeModal({ projectName, onSelect, allowedFormat, blocked, uploading, error, onDropFile, onClose }: InputTypeModalProps) {
    const [dragActive, setDragActive] = useState(false);
    const fileInputRef = useRef<HTMLInputElement | null>(null);

    // Counts nested drag-enter/leave events so the drop-zone highlight does not flicker.
    const dragDepthRef = useRef(0);

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !uploading) {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [uploading, onClose]);

    async function handleBrowseFile(event: ChangeEvent<HTMLInputElement>) {
        const file = event.target.files?.[0];

        // Reset the native input so the same file can be selected again after a failed import.
        event.target.value = "";
        if (!file) {
            return;
        }
        await onDropFile(file);
    }

    function handleDragEnter(event: DragEvent<HTMLDivElement>) {
        event.preventDefault();
        event.stopPropagation();

        if (blocked || uploading) {
            return;
        }

        dragDepthRef.current += 1;
        setDragActive(true);
    }

    function handleDragOver(event: DragEvent<HTMLDivElement>) {
        event.preventDefault();
        event.stopPropagation();
        if (blocked || uploading) {
            return;
        }

        event.dataTransfer.dropEffect = "copy";
    }

    function handleDragLeave(event: DragEvent<HTMLDivElement>) {
        event.preventDefault();
        event.stopPropagation();
        dragDepthRef.current = Math.max(0, dragDepthRef.current - 1);

        if (dragDepthRef.current === 0) {
            setDragActive(false);
        }
    }

    async function handleDrop(event: DragEvent<HTMLDivElement>) {
        event.preventDefault();
        event.stopPropagation();

        dragDepthRef.current = 0;
        setDragActive(false);
        if (blocked || uploading) {
            return;
        }

        const files = Array.from(event.dataTransfer.files);

        if (files.length === 0) {
            return;
        }

        //Network import intentionally accepts one primary network file at a time. TPO database files are handled by the complementary-database workflow.
        await onDropFile(files[0]);
    }

    function handleDropZoneKeyDown(event: ReactKeyboardEvent<HTMLDivElement>) {
        if (event.key !== "Enter" && event.key !== " ") {
            return;
        }

        event.preventDefault();
        if (!blocked && !uploading) {
            fileInputRef.current?.click();
        }
    }

    function allowedFormatMessage(): string {
        if (blocked) {
            return "Network import is currently blocked " + "for this project.";
        }

        if (allowedFormat === "DSG") {
            return "This project accepts DSG files only.";
        }

        if (allowedFormat === "TPO") {
            return "This project accepts TPO files only.";
        }

        return "Drop a .dsg or .tpo network file here.";
    }

    return createPortal(
        <div className="project-modal-backdrop"
            onMouseDown={event => {
                if (event.target === event.currentTarget && !uploading) {
                    onClose();
                }
            }}
        >
            <section className="project-modal input-type-modal" role="dialog"
                aria-modal="true" aria-labelledby="input-type-title">
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Network input</span>
                        <h2 id="input-type-title">Select input format</h2>
                        <p>Project: {projectName}</p>
                    </div>

                    <button type="button" className="project-modal__close"
                        aria-label="Close" disabled={uploading} onClick={onClose}>
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    <div className="input-type-grid">

                        <button type="button" className="input-type-card"
                            disabled={uploading || blocked || (allowedFormat !== null && allowedFormat !== "DSG")}
                            onClick={() => onSelect("DSG")}>
                            <strong>DSG</strong>
                            <span>Open an existing NeatWork design file.</span>
                            <small> .dsg</small>
                        </button>

                        <button type="button" className="input-type-card"
                            disabled={uploading || blocked || (allowedFormat !== null && allowedFormat !== "TPO")}
                            onClick={() => onSelect("TPO")}>
                            <strong>TPO</strong>
                            <span>Open a NeatWork topology input file.</span>
                            <small> .tpo</small>
                        </button>

                        <div className={dragActive
                            ? "network-file-drop-zone active" : uploading
                                ? "network-file-drop-zone busy"
                                : "network-file-drop-zone"}
                            role="button"
                            tabIndex={blocked || uploading ? -1 : 0}
                            aria-disabled={blocked || uploading}
                            onDragEnter={handleDragEnter}
                            onDragOver={handleDragOver}
                            onDragLeave={handleDragLeave}
                            onDrop={event => void handleDrop(event)}
                            onKeyDown={handleDropZoneKeyDown}
                            onClick={() => {

                                if (blocked || uploading) {
                                    return;
                                }
                                fileInputRef.current?.click();
                            }}
                        >

                            <input ref={fileInputRef}
                                type="file"
                                hidden
                                accept=".dsg,.tpo"
                                onChange={event => void handleBrowseFile(event)}
                            />
                            <div className="network-file-drop-zone__icon" aria-hidden="true">
                                ⇩
                            </div>
                            <strong>
                                {uploading ? "Importing network..." : dragActive
                                    ? "Release file to import"
                                    : "Drop network file here"
                                }
                            </strong>

                            <span>
                                {allowedFormatMessage()}
                            </span>

                            {!uploading && (
                                <small>or click to browse your computer</small>
                            )}
                        </div>

                        {error && (
                            <div className="project-modal__error" role="alert">{error}</div>
                        )}
                    </div>
                </div>

                <footer className="project-modal__footer">
                    <button type="button" className="project-modal__secondary"
                        disabled={uploading} onClick={onClose}>
                        Cancel
                    </button>
                </footer>
            </section>
        </div>,
        document.body
    );
}