#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo 'Usage: build-native-unix.sh TARGET_TRIPLE BUNDLES' >&2
  exit 2
fi

target="$1"
bundles="$2"
case "$(uname -s):$target" in
  Linux:x86_64-unknown-linux-gnu|Darwin:aarch64-apple-darwin|Darwin:x86_64-apple-darwin) ;;
  *) echo "Unsupported host/target pair: $(uname -s)/$target" >&2; exit 2 ;;
esac
if [[ "$(uname -s)" == Darwin ]]; then
  export MACOSX_DEPLOYMENT_TARGET=11.0
fi

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tauri_root="$workspace/apps/desktop/src-tauri"
source_root="$workspace/.local/occt-source"
build_root="$workspace/.local/occt-build-unix-$target"
occt_root="${FORMA_OCCT_ROOT:-$workspace/.local/occt-install-unix-$target}"

if [[ ! -f "$source_root/CMakeLists.txt" ]]; then
  mkdir -p "$(dirname "$source_root")"
  git clone --depth 1 --branch V8_0_1 https://github.com/Open-Cascade-SAS/OCCT.git "$source_root"
fi
if [[ ! -f "$occt_root/lib/libTKernel.a" ]]; then
  cmake -S "$source_root" -B "$build_root" \
    -DCMAKE_BUILD_TYPE=Release \
    -DINSTALL_DIR="$occt_root" \
    -DINSTALL_DIR_LAYOUT=Unix \
    -DINSTALL_DIR_WITH_VERSION=OFF \
    -DINSTALL_DIR_INCLUDE=include/opencascade \
    -DINSTALL_DIR_LIB=lib \
    -DBUILD_LIBRARY_TYPE=Static \
    -DBUILD_MODULE_Draw=OFF \
    -DBUILD_MODULE_Visualization=OFF \
    -DUSE_TK=OFF \
    -DUSE_FREETYPE=OFF \
    -DUSE_TBB=OFF \
    -DUSE_XLIB=OFF \
    -DUSE_OPENGL=OFF
  cmake --build "$build_root" --config Release --parallel "${FORMA_BUILD_JOBS:-2}"
  cmake --install "$build_root" --config Release
fi

if [[ ! -f "$occt_root/lib/libTKernel.a" ]]; then
  echo "Static OCCT installation is incomplete: $occt_root" >&2
  exit 1
fi

export FORMA_OCCT_ROOT="$occt_root"
cargo build --manifest-path "$tauri_root/Cargo.toml" \
  --release --locked --target "$target" --features native-occt --bin forma-cad-worker

target_root="${CARGO_TARGET_DIR:-$tauri_root/target}"
worker="$target_root/$target/release/forma-cad-worker"
test -x "$worker"
python3 "$workspace/scripts/check-native-unix.py" "$worker"
if [[ "${FORMA_NATIVE_TEST_ONLY:-0}" == '1' ]]; then
  cargo test --manifest-path "$tauri_root/Cargo.toml" \
    --locked --target "$target" --features native-occt --lib --test native_worker
  exit 0
fi

mkdir -p "$tauri_root/binaries" "$tauri_root/resources/occt"
cp "$worker" "$tauri_root/binaries/forma-cad-worker-$target"
chmod 755 "$tauri_root/binaries/forma-cad-worker-$target"
for license in LICENSE_LGPL_21.txt OCCT_LGPL_EXCEPTION.txt; do
  cp "$source_root/$license" "$tauri_root/resources/occt/$license"
done

cd "$workspace"
npm run tauri -w apps/desktop -- build \
  --target "$target" --features native-occt --bundles "$bundles" \
  --config "$tauri_root/tauri.native.unix.conf.json"
