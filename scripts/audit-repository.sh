#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

failures=0

report_failure() {
  printf 'AUDIT_REFUSED: %s\n' "$1" >&2
  failures=$((failures + 1))
}

for required in Cargo.toml rust-toolchain.toml README.md CONTRIBUTING.md SECURITY.md AGENTS.md; do
  if [[ ! -f "$required" ]]; then
    report_failure "required file missing: $required"
  fi
done

metadata_file="$(mktemp)"
trap 'rm -f "$metadata_file"' EXIT
cargo metadata --format-version 1 --no-deps >"$metadata_file"

python3 - "$repo_root" "$metadata_file" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
metadata = json.loads(pathlib.Path(sys.argv[2]).read_text())
errors: list[str] = []

for package in metadata["packages"]:
    manifest = pathlib.Path(package["manifest_path"]).resolve()
    if not manifest.is_relative_to(root):
        errors.append(f"manifest escapes repository: {manifest}")
    for dependency in package["dependencies"]:
        path = dependency.get("path")
        if path is None:
            continue
        resolved = pathlib.Path(path).resolve()
        if not resolved.is_relative_to(root):
            errors.append(
                f"dependency escapes repository: {package['name']} -> "
                f"{dependency['name']} ({resolved})"
            )

root_package = next(package for package in metadata["packages"] if package["name"] == "lsp-max")
if root_package.get("rust_version") != "1.87.0":
    errors.append(
        "lsp-max rust-version must be 1.87.0; observed "
        f"{root_package.get('rust_version')!r}"
    )

if errors:
    for error in errors:
        print(f"AUDIT_REFUSED: {error}", file=sys.stderr)
    raise SystemExit(1)
PY

if rg -n 'just setup|fetch sibling repos|depends on three siblings|\.\./lsp-types-max|\.\./wasm4pm-compat|\.\./wasm4pm' README.md CONTRIBUTING.md; then
  report_failure "documentation still requires machine-adjacent sibling repositories"
fi

if rg -n 'cargo run --bin lsp-max(\s|$)' README.md docs CONTRIBUTING.md; then
  report_failure "documentation references a nonexistent root lsp-max binary"
fi

if rg -n '(^|[^-])cargo publish([[:space:]]|$)' .github/workflows scripts --glob '*.yml' --glob '*.yaml' --glob '*.sh' | rg -v -- '--dry-run'; then
  report_failure "automated real cargo publish path detected"
fi

if rg -n 'continue-on-error:[[:space:]]*true|\|\|[[:space:]]*true' .github/workflows --glob '*.yml' --glob '*.yaml'; then
  report_failure "fail-open workflow construct detected"
fi

if [[ "$failures" -ne 0 ]]; then
  printf 'AUDIT_REFUSED: %d repository invariant(s) failed\n' "$failures" >&2
  exit 1
fi

printf 'AUDIT_ALIVE: repository topology, documentation, workflow, and publish boundaries verified\n'
