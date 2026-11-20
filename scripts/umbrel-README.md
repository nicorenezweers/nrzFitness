# nrzFitness - Umbrel-Paket

Dieser Zweig enthaelt nur fertige Dateien fuer die App `nrz-fitness` im nrzStore:

| Datei | Inhalt |
|-------|--------|
| `nrzfitness-amd64.gz`, `nrzfitness-arm64.gz` | Server (Linux, glibc 2.31) |
| `public.tar.gz` | Oberflaeche (WASM, CSS, Icons, Barcode-Scanner) |
| `SHA256SUMS` | Pruefsummen |

`hooks/pre-start` der App laedt sie per Tag `umbrel-<version>`. Erstellt mit
`scripts/umbrel-release` auf dem Zweig `main`.
