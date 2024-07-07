//! implementation of key generation.

use algebra::NTTField;
use fhe_core::{Parameters, SecretKeyPack};

/// Struct of key generation.
pub struct KeyGen;

impl KeyGen {
    /// Generate key pair
    #[inline]
    pub fn generate_secret_key<F: NTTField>(params: Parameters<F>) -> SecretKeyPack<F> {
        SecretKeyPack::new(params)
    }
}