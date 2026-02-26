# Sigillum - PDF Digital Signature

<img width="890" height="726" alt="image" src="https://github.com/user-attachments/assets/92cbd0a5-0f93-47f5-a0ae-e2106a19f91e" />

**Sigillum** is a personal tool for signing PDF documents. Generate a public key pair, sign a PDF, and verify a PDF. A watermark is also displayed in the top-left of the PDF document with the Signer's Name, Timestamp, and even Extras.

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
