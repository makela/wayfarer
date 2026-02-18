watch:
    DEV= cargo watch --no-gitignore -x "run"

tailwind:
    tailwindcss -i ./input.css -o ./dist/tailwind.css --watch

release:
    tailwindcss -i ./input.css -o ./dist/tailwind.css --minify
    cargo build --release
