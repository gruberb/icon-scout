# IconScout

This service is meant to run as a cron job or on a one-off basis. It reads a provided `websites.json` and fetches the favicons for each entry.

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

Either via `cargo run` inside the `icon-scout folder`, or via the binary: `./target/build/release/icon-scout`

### Example

```bash
     Running `target/debug/icon-scout`
2024-09-03T17:35:13.518692Z  INFO icon_scout::favicon: Checking: https://lobste.rs/touch-icon-144.png
2024-09-03T17:35:13.663589Z  INFO icon_scout::website: Favicon saved for https://lobste.rs as .png
2024-09-03T17:35:13.998476Z  INFO icon_scout::favicon: Checking: https://news.ycombinator.com/y18.svg
2024-09-03T17:35:14.067161Z  INFO icon_scout::favicon: Checking: https://theverge.com/icons/android_chrome_512x512.png
2024-09-03T17:35:14.126746Z  INFO icon_scout::favicon: Checking: https://github.githubassets.com/favicons/favicon.svg
2024-09-03T17:35:14.222306Z  INFO icon_scout::favicon: Checking: https://www.mozilla.org/media/img/favicons/mozilla/favicon-196x196.2af054fea211.png
2024-09-03T17:35:14.295554Z  INFO icon_scout::website: Favicon saved for https://github.com as .svg
2024-09-03T17:35:14.416995Z  INFO icon_scout::website: Favicon saved for https://news.ycombinator.com as .svg
2024-09-03T17:35:14.471524Z  INFO icon_scout::website: Favicon saved for https://theverge.com as .png
2024-09-03T17:35:14.651363Z  INFO icon_scout::website: Favicon saved for https://mozilla.org as .png
2024-09-03T17:35:14.651552Z  INFO icon_scout: Time elapsed: 1.391670208s
```
