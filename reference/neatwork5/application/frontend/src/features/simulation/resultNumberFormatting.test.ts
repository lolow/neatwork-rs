import { describe, expect, it } from "vitest";
import { formatAdaptiveNumber, formatRatioAsPercent } from "./resultNumberFormatting";


describe("formatAdaptiveNumber", () => {
    it("keeps ordinary values compact", () => {
        expect(formatAdaptiveNumber(12.3467)).toBe("12.35");
    });

    it("does not hide a small non-zero value", () => {
        expect(formatAdaptiveNumber(0.0047)).toBe("0.0047");
    });

    it("uses scientific notation for very small values", () => {
        expect(formatAdaptiveNumber(0.000031)).toBe("3.10e-5");
    });

    it("keeps real zero readable", () => {
        expect(formatAdaptiveNumber(0)).toBe("0.00");
    });

    it("does not turn small negative values into zero", () => {
        expect(formatAdaptiveNumber(-0.0047)).toBe("-0.0047");
    });
});

describe("formatRatioAsPercent", () => {
    it("converts a satisfaction ratio to percent", () => {
        expect(formatRatioAsPercent(0.95)).toBe("95.00%");
    });
    it("renders full satisfaction as one hundred percent", () => {
        expect(formatRatioAsPercent(1)).toBe("100.00%");
    });
});