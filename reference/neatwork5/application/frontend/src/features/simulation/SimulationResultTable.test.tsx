import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { SimulationResultTable } from "./SimulationResultTable";


const table = {
    key: "terminalSatisfaction",
    title: "Terminal Satisfaction",
    description: "Test table",
    columns: [
        {
            key: "terminalId",
            label: "Terminal",
        },
        {
            key: "minSat",
            label: "Minimum",
            unit: "%",
            numeric: true,
            format: "PERCENT" as const,
        },
    ],

    rows: [
        {
            terminalId: "P10",
            minSat: 0.4,
        },
        {
            terminalId: "P2",
            minSat: 0.9,
        },
    ],
};

it("renders satisfaction as percent", () => {
    render(<SimulationResultTable table={table} displayUnit={"m"} />);
    expect(screen.getByText("40.00%")).toBeInTheDocument();
    expect(screen.getByText("90.00%")).toBeInTheDocument();
});

it("sorts result rows by raw numeric value", async () => {
    const user = userEvent.setup();
    render(<SimulationResultTable table={table} displayUnit={"m"} />);
    await user.click(screen.getByRole("button", { name: /minimum/i }));
    const rows = screen.getAllByRole("row");
    // Header is row 0.
    expect(rows[1]).toHaveTextContent("P10");
    expect(rows[2]).toHaveTextContent("P2");
});