## 4. Split it into parts

```bash
skat split skills/<name>
skat --check
```

`skat split` writes `dev/skill/` and checks that the parts rebuild the draft byte for byte. `skat --check` exits 0 when `skills/<name>/` is up to date. On any other result, show the output and stop.

A `.md` file listed as copied, rather than split, either has one section or blank lines skat can't rebuild exactly. It still builds; it just stays one part.
