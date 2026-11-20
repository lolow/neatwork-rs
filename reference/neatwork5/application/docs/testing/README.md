# NeatWork automated testing

## Fast verification

```bash
./scripts/test-all.sh

## Hydraulic regression references

Regression reference JSON files are reviewed scientific baselines.

They must never be regenerated only to make a failing test pass.

A reference update is valid only when:

1. the numerical change is intentional;
2. the reason is documented;
3. the old/new results are reviewed;
4. the reference diff is committed with the solver change.