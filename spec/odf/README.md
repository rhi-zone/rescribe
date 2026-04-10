# ODF Schema Files

## ODF 1.2 (`odf-1.2.rnc`)

Converted from the OASIS ODF 1.2 OS RNG schema using `trang`.  
Original: <https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-schema.rng>

License: OASIS standard — redistributable with copyright notice preserved.  
Copyright (c) OASIS Open 2002–2011.

## ODF 1.3 (`odf-1.3.rnc`)

Not committed — "All Rights Reserved" in header without explicit redistribution grant.

To generate locally:

```bash
curl -sL https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-schema.rng \
  -o /tmp/odf-1.3.rng
trang /tmp/odf-1.3.rng spec/odf/odf-1.3.rnc
```

(`trang` is available via `nix develop` in this repo.)
