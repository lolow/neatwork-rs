#!/usr/bin/env bash
# Build the headless NeatWork 2024 harness.
# Output: reference/harness/build/neatwork-harness.jar
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../neatwork2024/src"
work="$here/build/src"
classes="$here/build/classes"
rm -rf "$here/build"
mkdir -p "$work" "$classes"

# Non-GUI part of the original sources, unchanged except as noted.
cp -r "$src/neatwork" "$work/"
rm -rf "$work/neatwork/gui" "$work/neatwork/Loader.java"
rm -f "$work/neatwork/solver/Solver.java" "$work/neatwork/solver/SolverClient.java" \
      "$work/neatwork/solver/SolverServer.java" "$work/neatwork/solver/IpoptSimulationProblem.java" \
      "$work/neatwork/file/FileManagerClient.java" "$work/neatwork/file/FileManagerServer.java" \
      "$work/neatwork/utils/ProgressCellRenderer.java" "$work/neatwork/utils/NeatworkAction.java" \
      "$work/neatwork/utils/IntegerInputVerifier.java" "$work/neatwork/utils/DoubleInputVerifier.java"
# SimulFlows imports Apache POI but never uses it.
sed -i '/import org\.apache\.poi/d' "$work/neatwork/core/SimulFlows.java"
# Harness files: stub Solver, MOSEK enums, CLI.
cp -r "$here/src/." "$work/"

javac -nowarn -encoding UTF-8 -d "$classes" $(find "$work" -name '*.java') 2>&1 | grep -v '^Note:' || true
# Resources: property bundles, databases, interpolation tables.
(cd "$work" && find . -type f ! -name '*.java' -exec cp --parents {} "$classes/" \;)
jar --create --file "$here/build/neatwork-harness.jar" --main-class neatwork.harness.Harness -C "$classes" .
echo "built $here/build/neatwork-harness.jar"
