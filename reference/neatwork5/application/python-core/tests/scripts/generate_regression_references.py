"""Generate reviewed hydraulic regression baselines.

This script is never executed automatically by pytest.
"""

from __future__ import annotations

import json

from tests.regression.regression_cases import (REGRESSION_CASES)
from tests.regression.regression_helpers import (canonicalize_regression_payload,run_regression_case)


def main() -> None:
    for case in REGRESSION_CASES:
        payload = (
            run_regression_case(
                dsg_path=case.dsg_path,
                scenario_count=case.scenario_count,
                seed=case.seed,
            )
        )
        payload = canonicalize_regression_payload(payload)
        case.reference_path.parent.mkdir(parents=True,exist_ok=True)
        case.reference_path.write_text(
            json.dumps(payload, indent=2, sort_keys=True, allow_nan=False)
            + "\n",
            encoding="utf-8",
        )
        print(f"Wrote {case.case_id}: {case.reference_path}")

if __name__ == "__main__":
    main()