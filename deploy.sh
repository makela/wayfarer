PATH=~/.cargo/bin:$PATH
mkdir -p dist
./create-feed.rs > dist/feed.xml
perseus deploy -e
wasm-opt -Oz -o ./dist/pkg/perseus_engine_bg.wasm ./dist/pkg/perseus_engine_bg.wasm
