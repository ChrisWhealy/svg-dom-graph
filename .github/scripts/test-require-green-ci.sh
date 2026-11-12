#!/usr/bin/env bash
# Checks require-green-ci.sh against a stand-in for the GitHub CLI that answers with runs written here, so that every outcome has a case.
#   .github/scripts/test-require-green-ci.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin=$(mktemp -d)
trap 'rm -rf "$bin"' EXIT
printf '#!/usr/bin/env bash\ncat "$FAKE_RUNS"\n' > "$bin/gh"
chmod +x "$bin/gh"
failures=0
expect() {  # expect <exit status> <description> <json>
  local want="$1" text="$2"
  FAKE_RUNS=$(mktemp); printf '%s' "$3" > "$FAKE_RUNS"; export FAKE_RUNS
  PATH="$bin:$PATH" "$here/require-green-ci.sh" abc123 > /dev/null 2>&1
  local got=$?
  rm -f "$FAKE_RUNS"
  if [ "$got" = "$want" ]; then echo "ok   $text"; else echo "FAIL $text (exit $got, wanted $want)"; failures=1; fi
}
expect 0 "one completed success is green" '[{"databaseId":1,"status":"completed","conclusion":"success","event":"push"}]'
expect 0 "several completed successes are green" '[{"databaseId":1,"status":"completed","conclusion":"success","event":"push"},{"databaseId":2,"status":"completed","conclusion":"success","event":"pull_request"}]'
expect 1 "a failed run is not green" '[{"databaseId":1,"status":"completed","conclusion":"failure","event":"push"}]'
expect 1 "one failure among successes is not green" '[{"databaseId":1,"status":"completed","conclusion":"success","event":"push"},{"databaseId":2,"status":"completed","conclusion":"failure","event":"push"}]'
expect 1 "a run that is still going is not green" '[{"databaseId":1,"status":"in_progress","conclusion":"","event":"push"}]'
expect 1 "no run at all is not green" '[]'
exit $failures
