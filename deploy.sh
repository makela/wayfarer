PATH=~/.cargo/bin:$PATH
perseus deploy
cp dist/styles.css pkg/dist/styles.css
./create-feed.rs > pkg/dist/feed.xml
wasm-opt -Oz -o ./pkg/dist/pkg/perseus_engine_bg.wasm ./pkg/dist/pkg/perseus_engine_bg.wasm
