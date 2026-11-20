import { describe, expect, it } from "vitest";
import { parseDomainEditorCell } from "./domainEditorValidation";

it(
    "rejects text in numeric columns",
    () => {

        const result =
            parseDomainEditorCell(
                "abc",
                "haut",
                "m",
                2
            );


        expect(
            result.ok
        ).toBe(
            false
        );
    }
);

it(
    "rejects probability above one",
    () => {

        const result =
            parseDomainEditorCell(
                "1.2",
                "openProbability",
                "m",
                0.4
            );


        expect(
            result.ok
        ).toBe(
            false
        );
    }
);

it(
    "keeps target flow in litres per second",
    () => {

        const result =
            parseDomainEditorCell(
                "3",
                "targetFlowLps",
                "ft",
                0.2
            );


        expect(
            result
        ).toEqual(
            {
                ok: true,
                value: 3,
            }
        );
    }
);

it(
    "converts edited feet back to canonical metres",
    () => {

        const result =
            parseDomainEditorCell(
                "3.280839895",
                "haut",
                "ft",
                0
            );


        expect(
            result.ok
        ).toBe(
            true
        );


        if (result.ok) {

            expect(
                result.value
            ).toBeCloseTo(
                1,
                8
            );
        }
    }
);

it(
    "does not coerce identifiers to numbers",
    () => {

        const result =
            parseDomainEditorCell(
                "001",
                "client",
                "m",
                "C1"
            );


        expect(
            result
        ).toEqual(
            {
                ok: true,
                value: "001",
            }
        );
    }
);

it(
    "rejects invalid node status",
    () => {

        const result =
            parseDomainEditorCell(
                "3",
                "statut",
                "m",
                1
            );


        expect(
            result.ok
        ).toBe(
            false
        );
    }
);