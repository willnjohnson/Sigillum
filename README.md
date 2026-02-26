# IMPORTANT NOTICE

This repo is purely a "toy-project" as an attempt to implement digital signatures. It only handles signing (integrity) and does not provide authenticity/ownership (i.e. certificates).

Furthermore, this implementation is NOT compatible with verification tools in Adobe Acrobat or other readers.
See FIPS 186-4: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-4.pdf

Other reasons not to use this application for major tasks:
- Randomness may NOT be random enough (uses urandom rather than non-determistic methods like mouse movement).
- This program has not be vetted by any established security company (and likely never will be).

What I would like to have, but is unlikely to ever get standardized:
- PGP key (sign with private key) as method of proving ownership

**As much as I despise Adobe, please do not roll your own crypto.**