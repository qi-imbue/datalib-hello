# datalib-hello

A minimal [datalib](https://github.com/imbue-ai/datalib) package, for
trying out packages. It adds one source type, **Hello**, whose only step
writes a greeting and the settings it was given into the library.

To add it to a library, open **Add source** (or **Packages** on the
Sources card) and paste:

```
github:qi-imbue/datalib-hello
```

## Layout

- `datalib-package.toml`: the manifest, without its `[artifacts]` table.
- `build/any/bin/hello-ingest`: the step's program, a shell script, so one
  build (`any`) serves every platform.

## Publishing a release

```sh
<datalib checkout>/scripts/pack_datalib_package.py . out
gh release create v<version> out/*
```

The format is described in datalib's
[`docs/dev/packages.md`](https://github.com/imbue-ai/datalib/blob/main/docs/dev/packages.md).
