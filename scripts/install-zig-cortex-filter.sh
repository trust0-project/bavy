#!/usr/bin/env bash
# rustc 1.98+ emits --fix-cortex-a53-843419 for aarch64-linux (Cortex-A53
# erratum 843419). GNU ld accepts it; Zig's bundled lld does not:
#   error: unsupported linker arg: --fix-cortex-a53-843419
# Shadow `zig` on PATH with a wrapper that drops the flag. Harmless on
# targets that never pass it (x86_64).
set -euo pipefail

real_zig="$(command -v zig)"
if [[ -z "$real_zig" ]]; then
  echo "zig not found on PATH" >&2
  exit 1
fi

wrap_dir="${RUNNER_TEMP:-$(pwd)/.zig-filter}/bin"
mkdir -p "$wrap_dir"

# Re-running would wrap the wrapper; keep the original binary.
if [[ "$real_zig" == "$wrap_dir/zig" ]]; then
  echo "zig Cortex-A53 filter already installed at $real_zig"
  exit 0
fi

quoted_zig="$(printf '%q' "$real_zig")"
cat > "$wrap_dir/zig" << WRAP
#!/usr/bin/env bash
set -euo pipefail
args=()
while ((\$#)); do
  case "\$1" in
    --fix-cortex-a53-843419|-Wl,--fix-cortex-a53-843419|-mfix-cortex-a53-843419)
      shift
      ;;
    -Xlinker)
      if [[ "\${2:-}" == "--fix-cortex-a53-843419" ]]; then
        shift 2
      else
        args+=("\$1")
        shift
      fi
      ;;
    *)
      args+=("\$1")
      shift
      ;;
  esac
done
exec ${quoted_zig} "\${args[@]}"
WRAP
chmod +x "$wrap_dir/zig"

echo "Wrapped zig ($real_zig) -> $wrap_dir/zig"

if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "$wrap_dir" >> "$GITHUB_PATH"
else
  echo "Add to PATH: export PATH=\"$wrap_dir:\$PATH\""
fi
