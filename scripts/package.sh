#!/bin/bash
# ClipStack 双端打包脚本
# 用法: ./scripts/package.sh
# 产物输出到 release-dist/
set -e
cd "$(dirname "$0")/.."

# 环境（Node22 + Rust + LLVM 的 llvm-rc/llvm-lib）
export PATH="$HOME/.nvm/versions/node/v22.22.3/bin:$HOME/.cargo/bin:/opt/homebrew/opt/llvm/bin:$PATH"

# 更新包签名密钥（~/.tauri/clipstack.key，不存在则跳过签名）
if [ -f "$HOME/.tauri/clipstack.key" ]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$HOME/.tauri/clipstack.key")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
fi

OUT="release-dist"
rm -rf "$OUT"
mkdir -p "$OUT"

echo "==> [1/4] 构建前端"
pnpm build

echo "==> [2/4] 构建 macOS release"
pnpm tauri build

APP="src-tauri/target/release/bundle/macos/ClipStack.app"

echo "==> [3/4] 重新签名 + 制作 dmg"
# Tauri ad-hoc 签名的资源封印不完整，直接分发会报"已损坏"，必须重签
codesign --force --deep --sign - "$APP"
codesign --verify --verbose=2 "$APP"
cp -R "$APP" "$OUT/"
hdiutil create -volname ClipStack -srcfolder "$OUT/ClipStack.app" -ov -format UDZO "$OUT/ClipStack-mac-arm64.dmg" >/dev/null

echo "==> [4/4] 交叉编译 Windows exe 并打包 zip"
(cd src-tauri && cargo xwin build --release --target x86_64-pc-windows-msvc)
cp src-tauri/target/x86_64-pc-windows-msvc/release/clipstack.exe "$OUT/"
(cd "$OUT" && zip -j -9 ClipStack-windows-x64.zip clipstack.exe && rm clipstack.exe)

echo ""
echo "==> 完成！产物在 $OUT/"
ls -lh "$OUT"
