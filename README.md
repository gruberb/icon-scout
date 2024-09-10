# IconScout

IconScout is a web service which is opening a POST endpoint on `/favicons`, which accepts a `JSON` file with a list of websites.

```bash
curl -X POST http://localhost:3000/favicons \
  -H "Content-Type: application/json" \
  --data-binary @websites.json \
  -o favicons.zip
```

Example JSON:
```json
[
  "https://yahoo.com",
  "https://theverge.com",
  "https://google.com",
  "https://accuweather.com"
]
```

or via `curl`:

```bash
curl -X POST http://localhost:3000/favicons \
     -H "Content-Type: application/json" \
     --data-raw '["https://google.com","https://yahoo.com","https://theverge.com"]' \
     -o favicons.zip
```

It parses the websites, fetches the favicons for them, *and sends back a ZIP file* as a response.

### Pre-requirements

Rust has to be installed
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Installation

- `git clone git@github.com:gruberb/icon-scout.git`
- `cd icon-scout`
- `cargo build --release`

### Running

Either via `cargo run` inside the `icon-scout` folder, or via the binary: `./target/build/release/icon-scout`

### Example

![alt text](https://github.com/gruberb/icon-scout/blob/main/example.png?raw=true)
