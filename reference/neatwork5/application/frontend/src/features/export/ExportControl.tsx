import { useEffect, useMemo, useRef, useState } from "react";
import type { ProjectExportAvailability, ProjectExportFormat } from "./types";
import { PROJECT_EXPORT_OPTIONS } from "./types";


type Props = {
    disabled: boolean;
    exporting: boolean;
    availability: ProjectExportAvailability[];
    onExport: (format: ProjectExportFormat) => Promise<void>;
};


/**
 * Export dropdown for the currently selected project.
 */
export function ExportControl({ disabled, exporting, availability, onExport }: Props) {

    const [open, setOpen] = useState(false);
    const containerRef = useRef<HTMLDivElement | null>(null);

    const availabilityByFormat = useMemo(
        () => new Map(
            availability.map(item => [item.format, item])
        ), [availability]
    );

    const hasAvailableFormat = availability.some(item => item.available);

    const exportDisabled = disabled || exporting || !hasAvailableFormat;

    useEffect(() => {

        function handleOutsideClick(event: MouseEvent) {

            if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
                setOpen(false);
            }
        }

        if (open) {
            document.addEventListener("mousedown", handleOutsideClick);
        }

        return () => {
            document.removeEventListener("mousedown", handleOutsideClick);
        };

    }, [open]);

    async function handleExport(format: ProjectExportFormat) {

        setOpen(false);
        await onExport(format);
    }

    return (
        <div className="export-control" ref={containerRef}>

            <button
                type="button"
                className="export-trigger"
                disabled={exportDisabled}
                aria-expanded={open}
                aria-haspopup="menu"
                onClick={() => setOpen(current => !current)}
            >
                {exporting ? "Exporting..." : "Export"}

                {!exporting && (
                    <span className="export-chevron" aria-hidden="true">▾</span>
                )}
            </button>

            {open && (
                <div className="export-menu" role="menu">

                    {PROJECT_EXPORT_OPTIONS.map(
                        option => {

                            const state = availabilityByFormat.get(option.value);
                            const available = state?.available === true;

                            return (
                                <button
                                    key={option.value}
                                    type="button"
                                    role="menuitem"
                                    className="export-menu-item"
                                    disabled={!available || exporting}
                                    onClick={() => handleExport(option.value)}
                                >
                                    {
                                        option.extension.replace(/^\./, "").toUpperCase()
                                    }
                                </button>
                            );
                        }
                    )}

                </div>
            )}

        </div>
    );
}