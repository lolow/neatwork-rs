export type ProjectExportFormat = "SIMULATION_JSON" | "SIMULATION_XLSX" | "DSG" | "INP";


export type ProjectExportAvailability = {
    format: ProjectExportFormat;
    available: boolean;
    reason: string | null;
};

export type ExportOption = {
    value: ProjectExportFormat;
    label: string;
    extension: string;
    description: string;
};

export const PROJECT_EXPORT_OPTIONS: ExportOption[] = [
    {
        value: "SIMULATION_JSON",
        label: "Simulation JSON",
        extension: ".json",
        description: "Canonical simulation statistics",
    },
    {
        value: "SIMULATION_XLSX",
        label: "Excel statistics",
        extension: ".xlsx",
        description: "Spreadsheet view of the latest simulation",
    },
    {
        value: "DSG",
        label: "NeatWork DSG",
        extension: ".dsg",
        description: "Latest available NeatWork design file",
    },
    {
        value: "INP",
        label: "EPANET INP",
        extension: ".inp",
        description: "Latest available EPANET network file",
    },
];