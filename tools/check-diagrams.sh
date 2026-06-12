#!/usr/bin/env bash
# Lightweight diagram gate — the §2 discipline, mechanically enforced.
# (A trimmed port of the Attestrum CLI repo's diagram-linter checks that
# matter for this repo; Mermaid parse is covered by render-diagrams.sh.)
#
# Usage: tools/check-diagrams.sh [--base <git-ref>]
#
# Hard failures:
#   1. Frontmatter — every docs/diagrams/**/*.md (except README.md) carries
#      title, models, source_of_truth, last_verified, diagram_type.
#   2. last_verified — the SHA must resolve to a commit in this repo.
#   3. Forward references — every path in `models:` exists in the worktree
#      (enforced for source_of_truth: code only; `diagram` contracts may
#      name code that doesn't exist yet).
#   4. Drift — with --base: if a commit range touches a path named in a
#      diagram's `models:`, that diagram must change in the same range.
#
# Warning (annotation only):
#   5. Freshness — last_verified older than 30 commits from HEAD.
set -euo pipefail

cd "$(dirname -- "${BASH_SOURCE[0]}")/.."

BASE=""
if [[ "${1:-}" == "--base" ]]; then
  BASE="${2:?--base needs a ref}"
fi

fail=0
err() { echo "FAIL: $*" >&2; fail=1; }
warn() {
  if [[ -n "${GITHUB_ACTIONS:-}" ]]; then echo "::warning::$*"; else echo "WARN: $*" >&2; fi
}

frontmatter() { # frontmatter <file> <key>
  awk -v key="$2" '
    NR==1 && $0=="---" { in_fm=1; next }
    in_fm && $0=="---" { exit }
    in_fm && index($0, key ":")==1 {
      sub("^" key ":[[:space:]]*", ""); gsub(/^"|"$/, ""); print; exit
    }
  ' "$1"
}

changed_files=""
if [[ -n "$BASE" ]] && git rev-parse --verify --quiet "$BASE^{commit}" >/dev/null; then
  changed_files="$(git diff --name-only "$BASE" HEAD)"
elif [[ -n "$BASE" ]]; then
  warn "base ref '$BASE' not found (new branch or force-push?) — drift check skipped"
fi

recent_shas="$(git log -30 --format=%h)"

while IFS= read -r -d '' diagram; do
  [[ "$(basename "$diagram")" == "README.md" ]] && continue
  rel="${diagram#./}"

  # 1. Frontmatter keys.
  for key in title models source_of_truth last_verified diagram_type; do
    if [[ -z "$(frontmatter "$diagram" "$key")" ]]; then
      err "$rel: missing frontmatter key '$key'"
    fi
  done

  sot="$(frontmatter "$diagram" source_of_truth)"
  lv_sha="$(frontmatter "$diagram" last_verified | awk '{print $1}')"
  models="$(frontmatter "$diagram" models)"

  # 2. last_verified resolves.
  if [[ -n "$lv_sha" ]] && ! git rev-parse --verify --quiet "${lv_sha}^{commit}" >/dev/null; then
    err "$rel: last_verified '$lv_sha' is not a commit in this repo"
  # 5. Freshness (warning only).
  elif [[ -n "$lv_sha" ]] && ! grep -qF "$lv_sha" <<<"$recent_shas"; then
    warn "$rel: last_verified $lv_sha is older than 30 commits — re-verify against current code"
  fi

  # 3. Forward references (code diagrams only).
  if [[ "$sot" == "code" && -n "$models" ]]; then
    IFS=',' read -ra paths <<<"$models"
    for p in "${paths[@]}"; do
      p="$(echo "$p" | xargs)" # trim
      [[ -e "$p" ]] || err "$rel: models path '$p' does not exist"
    done
  fi

  # 4. Drift against the commit range.
  if [[ -n "$changed_files" && -n "$models" ]]; then
    diagram_changed=0
    grep -qxF "$rel" <<<"$changed_files" && diagram_changed=1
    IFS=',' read -ra paths <<<"$models"
    for p in "${paths[@]}"; do
      p="$(echo "$p" | xargs)"
      if grep -q "^${p}" <<<"$changed_files" && [[ "$diagram_changed" -eq 0 ]]; then
        err "$rel: '$p' changed in this range but the diagram did not — update it (or bump last_verified) in the same change"
        break
      fi
    done
  fi
done < <(find docs/diagrams -type f -name '*.md' -print0)

if [[ "$fail" -ne 0 ]]; then
  echo
  echo "check-diagrams: FAILED — diagram-vs-code drift is a build break (CLAUDE.md §2)" >&2
  exit 1
fi
echo "check-diagrams: OK"
