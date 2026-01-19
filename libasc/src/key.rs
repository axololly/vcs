use std::{fmt::{Debug, Display, Formatter, Result as FmtResult}, hash::Hash, ops::{Deref, DerefMut}};

use crate::unwrap;

use ecdsa::{SigningKey, VerifyingKey, signature::{SignerMut, Verifier}};
use eyre::Result;
use p256::{NistP256};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

/// A private key used for creating signatures.
#[derive(Clone, Debug)]
pub struct PrivateKey(SigningKey<NistP256>);

impl PrivateKey {
    /// Create a new random [`PrivateKey`].
    #[allow(clippy::new_without_default, reason = "keys are randomly generated")]
    pub fn new() -> PrivateKey {
        let mut rng = rand::thread_rng();

        PrivateKey::random(&mut rng)
    }

    /// Create a [`PrivateKey`] from a given RNG device.
    pub fn random<C: CryptoRng + RngCore>(rng: &mut C) -> PrivateKey {
        PrivateKey(SigningKey::<NistP256>::random(rng))
    }

    /// Get the corresponding [`PublicKey`] to this [`PrivateKey`].
    pub fn public_key(&self) -> PublicKey {
        let raw: VerifyingKey<NistP256> = *self.0.verifying_key();

        raw.into()
    }

    /// Sign some data using the private key.
    pub fn sign(&mut self, bytes: &[u8]) -> Signature {
        Signature {
            inner: self.0.sign(bytes),
            key: self.public_key()
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PrivateKey> {
        let array: [u8; 32] = unwrap!(
            bytes.try_into(),
            "invalid length of bytes: {} (expected 32)", bytes.len()
        );

        let inner = SigningKey::<NistP256>::from_bytes(&array.into())?;

        Ok(PrivateKey(inner))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes().into()
    }
}

impl Deref for PrivateKey {
    type Target = SigningKey::<NistP256>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", hex::encode_upper(self.to_bytes()))
    }
}

impl Hash for PrivateKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.to_bytes());
    }
}

impl Serialize for PrivateKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let buf = ByteBuf::from(self.to_bytes());

        buf.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PrivateKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<PrivateKey, D::Error> {
        let buf = ByteBuf::deserialize(deserializer)?;
        
        PrivateKey::from_bytes(buf.as_slice())
            .map_err(serde::de::Error::custom)
    }
}

/// A public key used for verifying signatures.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Serialize)]
pub struct PublicKey(VerifyingKey<NistP256>);

impl PublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<PublicKey> {
        let raw = VerifyingKey::<NistP256>::from_sec1_bytes(bytes)?;

        Ok(raw.into())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_sec1_bytes().into_vec()
    }
}

impl From<VerifyingKey<NistP256>> for PublicKey {
    fn from(value: VerifyingKey<NistP256>) -> Self {
        PublicKey(value)
    }
}

impl Deref for PublicKey {
    type Target = VerifyingKey<NistP256>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PublicKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", hex::encode_upper(self.to_bytes()))
    }
}

impl Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.to_bytes());
    }
}

type RawSignature = ecdsa::Signature<NistP256>;

/// A signature with an attached verifying key.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Signature {
    inner: RawSignature,
    key: PublicKey
}

impl Signature {
    /// Check if the signature is valid, returning any errors if not.
    pub fn check(&self, data: &[u8]) -> Result<()> {
        self.key.verify(data, &self.inner)?;

        Ok(())
    }

    /// Check if the signature is valid, ignoring any errors
    /// unrelated to the authenticity of the signature.
    pub fn verify(&self, data: &[u8]) -> bool {
        self.check(data).is_ok()
    }

    /// Get the public key used to verify this signature.
    pub fn key(&self) -> PublicKey {
        self.key
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Signature> {
        let sig = rmp_serde::from_slice(bytes)?;

        Ok(sig)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).unwrap()
    }
}
