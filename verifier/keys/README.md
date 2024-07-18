## Key Files

This directory contains the key files used for signing & verifying verified BPF programs.

Note that these keys are committed to the repository for convenience, as they are used for testing and development purposes only. 
In a production environment, you should generate your own keys and keep them secure.

## Keys
- `ec_private_key.pem`: The private key used to sign BPF programs.
- `ec_public_key.pem`: The public key used to verify signed BPF programs.