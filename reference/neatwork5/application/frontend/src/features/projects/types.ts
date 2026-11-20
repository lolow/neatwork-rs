import type { DisplayLengthUnit } from "../domain/unitSystem";

export type ProjectSummary = {
    id: string,
    name: string,
    createdAt: string;
    sourceFormat: ProjectSourceFormat;
    // Project-wide hydraulic display unit.
    displayUnit: DisplayLengthUnit;
};

export type ProjectSourceFormat = "NONE" | "DSG" | "TPO" | "REVIEW_REQUIRED";
