import { describe, expect, it } from "vitest";
import { checkCatalogDeletion } from "./catalogIntegrity";


const domain = {
    tables: {
        diameters: [
            { ID: "D1", diameter_m: 0.02 },
            { ID: "D2", diameter_m: 0.03 },
        ],
        pipes: [
            { id: "A1", pred: "S1", succ: "J1", ID1: "D1", lon2: 0, ID2: "D2" },
        ],
        orifices: [{ diameter_m: 0.005 }],
        nodes: [
            { noeud: "P1", statut: 2, orif_ideal: 0.005, orif_com: 0.005 },
        ],
    },
};
it("blocks deletion of a referenced pipe diameter", () => {
    const result = checkCatalogDeletion(domain, "diameters", 0);
    expect(result.blocked).toBe(true);
    expect(result.references).toEqual(
        expect.arrayContaining([
            expect.objectContaining({ entityId: "A1", location: "Segment 1 diameter" }),
        ]));
});

it("ignores a stale segment 2 diameter when segment 2 length is zero", () => {
    const result = checkCatalogDeletion(domain, "diameters", 1);
    expect(result.blocked).toBe(false);
});

it("blocks deletion of an orifice used by a terminal", () => {
    const result = checkCatalogDeletion(domain, "orifices", 0);
    expect(result.blocked).toBe(true);
    expect(result.references.length).toBe(2);
});