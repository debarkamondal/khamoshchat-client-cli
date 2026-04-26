//! Signal Protocol integration via libsignal-dezire.
//!
//! Exposes the underlying x3dh and ratchet primitives through a
//! higher-level API suitable for the CLI client.

use anyhow::{Context, Result};
use libsignal_dezire::ratchet::{
    decrypt as ratchet_decrypt, encrypt as ratchet_encrypt,
    init_receiver_state, init_sender_state, RatchetState,
};
use libsignal_dezire::x3dh::{
    x3dh_initiator, x3dh_responder, PreKeyBundle, SignedPreKey,
    OneTimePreKey, X3DHInitResult, X3DHPrivateKey, X3DHPublicKey,
};
use libsignal_dezire::vxeddsa;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

pub use libsignal_dezire::ratchet::RatchetError;
pub use libsignal_dezire::x3dh::X3DHError;

// ─── Key Generation ──────────────────────────────────────────────────────────

/// A X3DH identity key pair (our long-term identity).
#[derive(Clone)]
pub struct IdentityKeyPair {
    pub public: X3DHPublicKey,
    pub private: X3DHPrivateKey,
}

impl IdentityKeyPair {
    /// Generate a new random identity key pair.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(&mut rand::rngs::OsRng);
        let public = PublicKey::from(&secret);
        let mut pub_prefixed = [0u8; 33];
        pub_prefixed[0] = 0x09;
        pub_prefixed[1..].copy_from_slice(public.as_bytes());
        let private_bytes = secret.to_bytes();
        Self {
            public: pub_prefixed,
            private: private_bytes,
        }
    }
}

/// Generate a new signed prekey from an identity key pair.
pub fn generate_signed_prekey(identity: &IdentityKeyPair, key_id: u32) -> SignedPreKey {
    // Generate a fresh DH key pair for the signed prekey
    let dh_secret = StaticSecret::random_from_rng(&mut rand::rngs::OsRng);
    let dh_public = PublicKey::from(&dh_secret);
    let mut signed_pub = [0u8; 33];
    signed_pub[0] = 0x09;
    signed_pub[1..].copy_from_slice(dh_public.as_bytes());

    // Sign the public key using VXEdDSA: vxeddsa_sign(key, message)
    // The message to sign is the encoded public key bytes.
    let output = vxeddsa::vxeddsa_sign(&identity.private, &signed_pub)
        .expect("VXEdDSA signing failed");

    SignedPreKey {
        id: key_id,
        public_key: signed_pub,
        signature: output.signature, // [u8; 96]
    }
}

/// Generate a batch of one-time prekeys.
pub fn generate_prekeys(start_id: u32, count: u32) -> Vec<(u32, OneTimePreKey)> {
    (0..count)
        .map(|i| {
            let secret = StaticSecret::random_from_rng(&mut rand::rngs::OsRng);
            let public = PublicKey::from(&secret);
            let mut pub_bytes = [0u8; 33];
            pub_bytes[0] = 0x09;
            pub_bytes[1..].copy_from_slice(public.as_bytes());
            (start_id + i, OneTimePreKey { id: start_id + i, public_key: pub_bytes })
        })
        .collect()
}

// ─── X3DH Session Initiation ─────────────────────────────────────────────────

/// Alice initiates X3DH with Bob's PreKeyBundle.
pub fn x3dh_alice_initiate(
    our_identity_private: &X3DHPrivateKey,
    bundle: &PreKeyBundle,
) -> Result<X3DHInitResult, X3DHError> {
    x3dh_initiator(our_identity_private, bundle)
}

/// Bob processes Alice's X3DH message.
pub fn x3dh_bob_respond(
    our_identity_private: &X3DHPrivateKey,
    our_signed_prekey_private: &X3DHPrivateKey,
    our_one_time_prekey_private: Option<&X3DHPrivateKey>,
    alice_identity_public: &X3DHPublicKey,
    alice_ephemeral_public: &X3DHPublicKey,
) -> Result<[u8; 32], X3DHError> {
    x3dh_responder(
        our_identity_private,
        our_signed_prekey_private,
        our_one_time_prekey_private,
        alice_identity_public,
        alice_ephemeral_public,
    )
}

// ─── Session State Serialization ────────────────────────────────────────────

/// Serializable snapshot of a RatchetState for persistence.
#[derive(Serialize, Deserialize)]
pub struct SerializedSession {
    pub state: Vec<u8>,
}

impl SerializedSession {
    pub fn new(state: &RatchetState) -> Result<Self> {
        let json = serde_json::to_string(state)
            .context("Serialize ratchet state")?;
        Ok(Self {
            state: json.into_bytes(),
        })
    }

    pub fn load(state_bytes: &[u8]) -> Result<RatchetState> {
        serde_json::from_slice(state_bytes)
            .context("Deserialize ratchet state")
    }
}

// ─── Double Ratchet ─────────────────────────────────────────────────────────

/// Initialize a sender (Alice) session from a shared secret.
pub fn init_sender(shared_secret: [u8; 32], remote_public: &[u8; 32]) -> Result<RatchetState, RatchetError> {
    let pk = PublicKey::from(*remote_public);
    init_sender_state(shared_secret, pk)
}

/// Initialize a receiver (Bob) session from a shared secret and our DH key pair.
/// Note: init_receiver_state returns RatchetState directly (not Result).
pub fn init_receiver(shared_secret: [u8; 32], our_dh_private: [u8; 32], our_dh_public: [u8; 32]) -> RatchetState {
    let secret = StaticSecret::from(our_dh_private);
    let public = PublicKey::from(our_dh_public);
    init_receiver_state(shared_secret, (secret, public))
}

/// Encrypt a plaintext. Returns (encrypted_header, ciphertext).
pub fn encrypt_msg(
    state: &mut RatchetState,
    plaintext: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), RatchetError> {
    ratchet_encrypt(state, plaintext, &[])
}

/// Decrypt a message given its header and ciphertext.
pub fn decrypt_msg(
    state: &mut RatchetState,
    enc_header: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, RatchetError> {
    ratchet_decrypt(state, enc_header, ciphertext, &[])
}
