# Portability fixtures

`archive-v1-contract.json` is the language-neutral wire contract for
`.mtgonotes` format version 1. The Rust archive tests generate deterministic
valid, wrong-passphrase, truncated, checksum-invalid, and unsupported-version
archives from fixed salt and nonce material, then verify the contract without
checking binary ciphertext into source control.

Archives contain canonical logical records only. SQLCipher database pages,
database keys, DPAPI material, provider consent, operation journals, and
machine-bound secrets are outside the format.
