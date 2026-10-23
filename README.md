# datalib-hello

A minimal [datalib](https://github.com/imbue-ai/datalib) package, for
trying out packages. It adds:

- a source type, **Hello**, whose one step writes a greeting and the
  settings it was given into the library;
- an applet with a **Greetings** card, which lists what each Hello source
  last wrote.

Both are small Rust programs, released as a build for each platform
datalib runs on, so the package also shows how one ships compiled code.

To add it to a library, open **Add source** (or **Packages** on the
Sources card) and paste:

```
github:qi-imbue/datalib-hello
```

## Layout

- `datalib-package.toml`: the manifest, without its `[artifacts]` table.
  The release adds that table, naming each platform's archive by sha256.
- `src/bin/hello-ingest.rs`: the step (datalib's `docs/dev/step_protocol.md`).
- `src/bin/hello-greetings.rs` and `src/greetings.js`: the applet and its
  card (datalib's `docs/dev/applets.md`).
- `tools/pack_datalib_package.py`: a copy of datalib's
  `scripts/pack_datalib_package.py`.

## Publishing a release

Bump the version in `datalib-package.toml` and `Cargo.toml`, then push a
tag:

```sh
git tag v0.2.0 && git push origin v0.2.0
```

`.github/workflows/release.yml` builds both programs for the five
platforms, packs one archive per platform and publishes them, with the
manifest, as that tag's release. `github:qi-imbue/datalib-hello` always
means the latest release.

The format is described in datalib's
[`docs/dev/packages.md`](https://github.com/imbue-ai/datalib/blob/main/docs/dev/packages.md).
