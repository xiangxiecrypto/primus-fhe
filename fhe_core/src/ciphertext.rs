/// LWE Ciphertext
pub type LWECiphertext = lattice::LWE<crate::LWEModulusType>;

/// RLWE Ciphertext
pub type RLWECiphertext<F> = lattice::RLWE<F>;

/// NTT version RLWE Ciphertext
pub type NTTRLWECiphertext<F> = lattice::NTTRLWE<F>;

/// NTRU Ciphertext
pub type NTRUCiphertext<F> = lattice::NTRU<F>;

/// NTT version NTRU Ciphertext
pub type NTTNTRUCiphertext<F> = lattice::NTTNTRU<F>;