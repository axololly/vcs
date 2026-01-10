use std::{fmt::{Debug, Display, Formatter, Result as FmtResult}, ops::{Deref, DerefMut}};

use crate::unwrap;

use ecdsa::{SigningKey, VerifyingKey, signature::{SignerMut, Verifier, rand_core::CryptoRngCore}};
use eyre::Result;
use p256::{NistP256, elliptic_curve::{NonZeroScalar, PublicKey as InnerVerifyingKey, ScalarPrimitive}};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize, de};
use serde_bytes::ByteBuf;

/// The raw version of [`PrivateKey`].
/// 
/// The layout of this struct is identical to that of [`SigningKey`],
/// and is used to add custom [`serde`] logic.
#[derive(Clone, Copy)]
pub struct RawPrivateKey {
    _secret_scalar: NonZeroScalar<NistP256>,
    verifying_key: VerifyingKey<NistP256>
}

impl RawPrivateKey {
    /// Generate a random [`RawPrivateKey`] with the given RNG device.
    pub fn random(rng: &mut impl CryptoRngCore) -> RawPrivateKey {
        SigningKey::<NistP256>::random(rng).into()
    }

    /// Reconstruct a [`RawPrivateKey`] from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<RawPrivateKey> {
        let raw = SigningKey::<NistP256>::from_bytes(bytes.into())?;
        
        Ok(raw.into())
    }
}

impl From<SigningKey<NistP256>> for RawPrivateKey {
    fn from(value: SigningKey<NistP256>) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for RawPrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "RawPrivateKey {{ inner: SigningKey {{ secret_scalar: ..., verifying_key: {:?} }} }}", self.verifying_key)
    }
} 

impl Deref for RawPrivateKey {
    type Target = SigningKey<NistP256>;

    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl Serialize for RawPrivateKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let bytes: &[u8] = &self.to_bytes();

        let buf = ByteBuf::from(bytes);

        buf.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RawPrivateKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes = ByteBuf::deserialize(deserializer)?;

        let priv_key = PrivateKey::from_bytes(&bytes)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(&bytes), &"a valid private key"))?;

        let raw: RawPrivateKey = priv_key.0;

        Ok(raw)
    }
}

/// A private key used for creating signatures.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct PrivateKey(RawPrivateKey);

impl PrivateKey {
    /// Create a new random [`PrivateKey`].
    #[allow(clippy::new_without_default, reason = "keys are randomly generated")]
    pub fn new() -> PrivateKey {
        let mut rng = rand::thread_rng();

        PrivateKey::random(&mut rng)
    }

    /// Create a [`PrivateKey`] from a given RNG device.
    pub fn random<C: CryptoRng + RngCore>(rng: &mut C) -> PrivateKey {
        PrivateKey(RawPrivateKey::random(rng))
    }

    /// Get the corresponding [`PublicKey`] to this [`PrivateKey`].
    pub fn public_key(&self) -> PublicKey {
        let raw: RawPublicKey = (*self.0.verifying_key()).into();

        raw.into()
    }

    /// Sign some data using the private key.
    pub fn sign(&mut self, bytes: &[u8]) -> Signature {
        let raw_key: &mut RawPrivateKey = &mut *self;
        
        let lib_sig: ecdsa::Signature<NistP256> = raw_key.sign(bytes);

        let raw_sig: RawSignature = lib_sig.into();

        Signature {
            inner: raw_sig,
            key: self.public_key()
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<PrivateKey> {
        let array: [u8; 32] = unwrap!(
            bytes.try_into(),
            "invalid length of bytes: {} (expected 32)", bytes.len()
        );

        let inner = RawPrivateKey::from_bytes(&array)?;

        Ok(PrivateKey(inner))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes().into()
    }
}

impl Deref for PrivateKey {
    type Target = RawPrivateKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RawPrivateKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl DerefMut for PrivateKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<RawPrivateKey> for PrivateKey {
    fn from(value: RawPrivateKey) -> Self {
        PrivateKey(value)
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

/// The raw version of [`PublicKey`].
/// 
/// The layout of this struct is identical to that of [`VerifyingKey`],
/// and is used to add custom [`serde`] logic.
#[derive(Clone, Copy, Debug)]
pub struct RawPublicKey {
    _inner: InnerVerifyingKey<NistP256>
}

impl RawPublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<RawPublicKey> {
        let raw = VerifyingKey::<NistP256>::from_sec1_bytes(bytes)?;
        
        Ok(raw.into())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_sec1_bytes().into_vec()
    }
}

impl Deref for RawPublicKey {
    type Target = VerifyingKey<NistP256>;

    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl DerefMut for RawPublicKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl From<VerifyingKey<NistP256>> for RawPublicKey {
    fn from(value: VerifyingKey<NistP256>) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl Serialize for RawPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let bytes: &[u8] = &self.to_bytes();

        let buf = ByteBuf::from(bytes);

        buf.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RawPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes = ByteBuf::deserialize(deserializer)?;

        let pub_key = PublicKey::from_bytes(&bytes)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(&bytes), &"a valid private key"))?;

        let raw: RawPublicKey = pub_key.0;

        Ok(raw)
    }
}

/// A public key used for verifying signatures.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct PublicKey(RawPublicKey);

impl PublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<PublicKey> {
        let raw = RawPublicKey::from_bytes(bytes)?;

        Ok(raw.into())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_sec1_bytes().into_vec()
    }
}

impl From<RawPublicKey> for PublicKey {
    fn from(value: RawPublicKey) -> Self {
        PublicKey(value)
    }
}

impl Deref for PublicKey {
    type Target = RawPublicKey;

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

/// A bit-identical copy of [`VerifyingKey`].
#[derive(Clone, Debug, PartialEq)]
pub struct RawSignature {
    _r: ScalarPrimitive<NistP256>,
    _s: ScalarPrimitive<NistP256>
}

impl RawSignature {
    pub fn from_bytes(bytes: &[u8]) -> Result<RawSignature> {
        let inner = ecdsa::Signature::<NistP256>::from_bytes(bytes.into())?;

        let signature = unsafe { std::mem::transmute::<ecdsa::Signature<NistP256>, RawSignature>(inner) };

        Ok(signature)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl Deref for RawSignature {
    type Target = ecdsa::Signature<NistP256>;

    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl DerefMut for RawSignature {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}

impl From<ecdsa::Signature<NistP256>> for RawSignature {
    fn from(value: ecdsa::Signature<NistP256>) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl Serialize for RawSignature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let bytes: &[u8] = &self.to_bytes();

        let buf = ByteBuf::from(bytes);

        buf.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RawSignature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes = ByteBuf::deserialize(deserializer)?;

        let signature = RawSignature::from_bytes(&bytes)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(&bytes), &"a valid signature"))?;

        Ok(signature)
    }
}

/// A signature with an attached verifying key.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Signature {
    inner: RawSignature,
    key: PublicKey
}

impl Signature {
    /// Check if the signature is valid, returning any errors if not.
    pub fn check(&self, data: &[u8]) -> Result<()> {
        (**self.key).verify(data, &*self.inner)?;

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