# datalib-hello

A small [datalib](https://github.com/imbue-ai/datalib) package, for
trying out packages. It adds a source type, **Hello**. Each sync of a
Hello source:

1. **ingest** appends the greeting it was given, and when, to
   `<source>/ingest/greetings.jsonl`;
2. **render_markdown** writes one document per greeting into the
   source's render store, so the greetings show up in the grid and in
   search;
3. **render_time_series** writes two time series, *Greeting length* and
   *Greetings per day*, which show up in the Time series card.

Each Hello source also gets a **Greetings** card, served by an applet,
that lists what that source has written.

All of it is Rust, released as a build for each platform datalib runs
on, so the package also shows how one ships compiled code.

To add it to a library, open **Packages** on the Sources card (or the
foot of **Add source**) and paste:

```
github:qi-imbue/datalib-hello
```

## Layout

- `datalib-package.toml`: the manifest, without its `[artifacts]` table.
  The release adds that table, naming each platform's archive by sha256.
- `src/bin/hello-ingest.rs`, `hello-render.rs`, `hello-time-series.rs`:
  the steps (datalib's `docs/dev/step_protocol.md`).
- `src/bin/hello-greetings.rs` and `src/greetings.js`: the applet and its
  card (datalib's `docs/dev/applets.md`).
- `datalib-sdk/`: what a program outside datalib needs to write the
  stores datalib's indexes read: the doltlite build datalib pins, the
  render store and its `.md` files, the time-series store and its chunk
  encoding, ids in datalib's layout, and the step protocol's events. It
  is a copy of datalib's formats, kept as its own crate so it can become
  a published one. datalib's `docs/dev/packages.md` §"Rendering" says
  what has to match.
- `tools/pack_datalib_package.py`: a copy of datalib's
  `scripts/pack_datalib_package.py`.

## Building

`cargo build --release`. The first build downloads doltlite's
amalgamation (checked against a pinned sha256) and compiles it, which
takes a minute; set `DOLTLITE_AMALGAMATION_DIR` to a directory holding
`doltlite.c` to build offline.

## Publishing a release

Bump the version in `datalib-package.toml` and `Cargo.toml`, then push a
tag:

```sh
git tag v0.3.0 && git push origin v0.3.0
```

`.github/workflows/release.yml` builds the programs for the five
platforms, packs one archive per platform and publishes them, with the
manifest, as that tag's release. `github:qi-imbue/datalib-hello` always
means the latest release.

The format is described in datalib's
[`docs/dev/packages.md`](https://github.com/imbue-ai/datalib/blob/main/docs/dev/packages.md).
