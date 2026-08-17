use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::domain::{RepoError, UtcMillis};
use crate::operations::CancellationToken;

pub const ARCHIVE_EXTENSION: &str = "mtgonotes";
pub const ARCHIVE_FORMAT_VERSION: u16 = 1;
pub const ARCHIVE_MAGIC: &[u8; 8] = b"MTGONTS1";
pub const STREAM_CHUNK_BYTES: usize = 64 * 1024;
pub const MAX_ENVELOPE_BYTES: usize = 16 * 1024;
pub const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
pub const KDF_MEMORY_KIB: u32 = 32 * 1024;
pub const KDF_ITERATIONS: u32 = 3;
pub const KDF_PARALLELISM: u32 = 1;

const FINAL_FRAME: u8 = 1;
const DATA_FRAME: u8 = 0;
const TAG_BYTES: usize = 16;
const HEADER_CHECKSUM_BYTES: usize = 32;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfContract {
    pub algorithm: String,
    pub version: u32,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub salt: [u8; 16],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherContract {
    pub algorithm: String,
    pub chunk_bytes: u32,
    pub nonce_prefix: [u8; 4],
    pub key_check: [u8; 16],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEnvelope {
    pub format_version: u16,
    pub kdf: KdfContract,
    pub cipher: CipherContract,
}

impl ArchiveEnvelope {
    pub fn random() -> Result<Self, RepoError> {
        let mut salt = [0_u8; 16];
        let mut nonce_prefix = [0_u8; 4];
        getrandom::fill(&mut salt).map_err(|_| RepoError::InvalidBackup)?;
        getrandom::fill(&mut nonce_prefix).map_err(|_| RepoError::InvalidBackup)?;
        Ok(Self::with_material(salt, nonce_prefix))
    }

    pub fn with_material(salt: [u8; 16], nonce_prefix: [u8; 4]) -> Self {
        Self {
            format_version: ARCHIVE_FORMAT_VERSION,
            kdf: KdfContract {
                algorithm: "argon2id".to_owned(),
                version: 0x13,
                memory_kib: KDF_MEMORY_KIB,
                iterations: KDF_ITERATIONS,
                parallelism: KDF_PARALLELISM,
                salt,
            },
            cipher: CipherContract {
                algorithm: "chacha20poly1305_ietf".to_owned(),
                chunk_bytes: STREAM_CHUNK_BYTES as u32,
                nonce_prefix,
                key_check: [0; 16],
            },
        }
    }

    fn validate(&self) -> Result<(), RepoError> {
        if self.format_version != ARCHIVE_FORMAT_VERSION
            || self.kdf.algorithm != "argon2id"
            || self.kdf.version != 0x13
            || self.kdf.memory_kib != KDF_MEMORY_KIB
            || self.kdf.iterations != KDF_ITERATIONS
            || self.kdf.parallelism != KDF_PARALLELISM
            || self.cipher.algorithm != "chacha20poly1305_ietf"
            || self.cipher.chunk_bytes != STREAM_CHUNK_BYTES as u32
        {
            return Err(RepoError::InvalidBackup);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveManifest {
    pub format_version: u16,
    pub created_at: UtcMillis,
    pub schema_min: i64,
    pub schema_max: i64,
    pub record_count: u64,
    pub table_counts: BTreeMap<String, u64>,
    pub table_hashes: BTreeMap<String, String>,
    pub classifier_provenance: Vec<ClassifierProvenance>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierProvenance {
    pub version: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalRecord {
    pub table: String,
    pub key: String,
    pub values: Vec<CanonicalValue>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CanonicalValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum ArchiveItem {
    Record(CanonicalRecord),
    Manifest(ArchiveManifest),
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedArchive {
    pub envelope: ArchiveEnvelope,
    pub manifest: ArchiveManifest,
    pub archive_sha256: String,
}

pub struct ManifestAccumulator {
    counts: BTreeMap<String, u64>,
    hashers: BTreeMap<String, Sha256>,
    total: u64,
}

impl Default for ManifestAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestAccumulator {
    pub fn new() -> Self {
        Self {
            counts: BTreeMap::new(),
            hashers: BTreeMap::new(),
            total: 0,
        }
    }

    pub fn push(&mut self, record: &CanonicalRecord) -> Result<Vec<u8>, RepoError> {
        let mut encoded = serde_json::to_vec(&ArchiveItem::Record(record.clone()))
            .map_err(|_| RepoError::InvalidBackup)?;
        encoded.push(b'\n');
        *self.counts.entry(record.table.clone()).or_default() += 1;
        self.hashers
            .entry(record.table.clone())
            .or_default()
            .update(&encoded);
        self.total = self.total.checked_add(1).ok_or(RepoError::InvalidBackup)?;
        Ok(encoded)
    }

    pub fn finish(
        self,
        created_at: UtcMillis,
        schema_version: i64,
        classifier_provenance: Vec<ClassifierProvenance>,
    ) -> ArchiveManifest {
        ArchiveManifest {
            format_version: ARCHIVE_FORMAT_VERSION,
            created_at,
            schema_min: schema_version,
            schema_max: schema_version,
            record_count: self.total,
            table_counts: self.counts,
            table_hashes: self
                .hashers
                .into_iter()
                .map(|(table, hasher)| (table, hex_digest(hasher.finalize())))
                .collect(),
            classifier_provenance,
        }
    }
}

pub struct ArchiveWriter<W: Write> {
    output: W,
    cipher: ChaCha20Poly1305,
    envelope_digest: [u8; 32],
    nonce_prefix: [u8; 4],
    counter: u64,
    pending: Vec<u8>,
}

impl<W: Write> ArchiveWriter<W> {
    pub fn new(
        mut output: W,
        passphrase: &str,
        envelope: ArchiveEnvelope,
    ) -> Result<Self, RepoError> {
        envelope.validate()?;
        if passphrase.is_empty() {
            return Err(RepoError::InvalidRequest);
        }
        let key = derive_key(passphrase, &envelope.kdf)?;
        let mut envelope = envelope;
        envelope.cipher.key_check = key_check(&key[..]);
        let envelope_bytes = serde_json::to_vec(&envelope).map_err(|_| RepoError::InvalidBackup)?;
        let envelope_length =
            u32::try_from(envelope_bytes.len()).map_err(|_| RepoError::InvalidBackup)?;
        output
            .write_all(ARCHIVE_MAGIC)
            .and_then(|_| output.write_all(&envelope_length.to_be_bytes()))
            .and_then(|_| output.write_all(&envelope_bytes))
            .and_then(|_| output.write_all(&Sha256::digest(&envelope_bytes)))
            .map_err(|_| RepoError::DestinationUnwritable)?;
        let cipher =
            ChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| RepoError::InvalidBackup)?;
        let envelope_digest: [u8; 32] = Sha256::digest(&envelope_bytes).into();
        Ok(Self {
            output,
            cipher,
            envelope_digest,
            nonce_prefix: envelope.cipher.nonce_prefix,
            counter: 0,
            pending: Vec::with_capacity(STREAM_CHUNK_BYTES * 2),
        })
    }

    pub fn write_plaintext(&mut self, bytes: &[u8]) -> Result<(), RepoError> {
        self.pending.extend_from_slice(bytes);
        while self.pending.len() > STREAM_CHUNK_BYTES {
            let remainder = self.pending.split_off(STREAM_CHUNK_BYTES);
            let chunk = std::mem::replace(&mut self.pending, remainder);
            self.write_frame(&chunk, false)?;
        }
        Ok(())
    }

    pub fn finish(mut self, manifest: &ArchiveManifest) -> Result<W, RepoError> {
        let mut manifest_bytes = serde_json::to_vec(&ArchiveItem::Manifest(manifest.clone()))
            .map_err(|_| RepoError::InvalidBackup)?;
        manifest_bytes.push(b'\n');
        self.write_plaintext(&manifest_bytes)?;
        let final_chunk = std::mem::take(&mut self.pending);
        self.write_frame(&final_chunk, true)?;
        self.output
            .flush()
            .map_err(|_| RepoError::DestinationUnwritable)?;
        Ok(self.output)
    }

    fn write_frame(&mut self, plaintext: &[u8], final_frame: bool) -> Result<(), RepoError> {
        let flag = if final_frame { FINAL_FRAME } else { DATA_FRAME };
        let nonce_bytes = nonce_bytes(self.nonce_prefix, self.counter);
        let aad = frame_aad(self.envelope_digest, self.counter, flag);
        let nonce =
            Nonce::try_from(nonce_bytes.as_slice()).map_err(|_| RepoError::InvalidBackup)?;
        let ciphertext = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| RepoError::InvalidBackup)?;
        let length = u32::try_from(ciphertext.len()).map_err(|_| RepoError::InvalidBackup)?;
        self.output
            .write_all(&[flag])
            .and_then(|_| self.output.write_all(&length.to_be_bytes()))
            .and_then(|_| self.output.write_all(&ciphertext))
            .map_err(|_| RepoError::DestinationUnwritable)?;
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(RepoError::InvalidBackup)?;
        Ok(())
    }
}

pub fn write_archive(
    path: &Path,
    passphrase: &str,
    envelope: ArchiveEnvelope,
    records: impl IntoIterator<Item = CanonicalRecord>,
    created_at: UtcMillis,
    schema_version: i64,
    classifier_provenance: Vec<ClassifierProvenance>,
) -> Result<ArchiveManifest, RepoError> {
    let file = File::create(path).map_err(|_| RepoError::DestinationUnwritable)?;
    let mut writer = ArchiveWriter::new(BufWriter::new(file), passphrase, envelope)?;
    let mut accumulator = ManifestAccumulator::new();
    for record in records {
        let encoded = accumulator.push(&record)?;
        writer.write_plaintext(&encoded)?;
    }
    let manifest = accumulator.finish(created_at, schema_version, classifier_provenance);
    let mut output = writer.finish(&manifest)?;
    output
        .flush()
        .map_err(|_| RepoError::DestinationUnwritable)?;
    output
        .get_ref()
        .sync_all()
        .map_err(|_| RepoError::DestinationUnwritable)?;
    Ok(manifest)
}

pub fn verify_archive(path: &Path, passphrase: &str) -> Result<VerifiedArchive, RepoError> {
    verify_archive_cancellable(path, passphrase, None)
}

pub fn verify_archive_with_cancellation(
    path: &Path,
    passphrase: &str,
    cancellation: &CancellationToken,
) -> Result<VerifiedArchive, RepoError> {
    verify_archive_cancellable(path, passphrase, Some(cancellation))
}

fn verify_archive_cancellable(
    path: &Path,
    passphrase: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<VerifiedArchive, RepoError> {
    let archive_sha256 = hash_file(path)?;
    let mut manifest = None;
    let mut accumulator = ManifestAccumulator::new();
    let envelope = read_archive(path, passphrase, cancellation, |item| match item {
        ArchiveItem::Record(record) => {
            if manifest.is_some() {
                return Err(RepoError::InvalidBackup);
            }
            accumulator.push(&record)?;
            Ok(())
        }
        ArchiveItem::Manifest(candidate) => {
            if manifest.replace(candidate).is_some() {
                return Err(RepoError::InvalidBackup);
            }
            Ok(())
        }
    })?;
    let manifest = manifest.ok_or(RepoError::InvalidBackup)?;
    let expected = accumulator.finish(
        manifest.created_at,
        manifest.schema_min,
        manifest.classifier_provenance.clone(),
    );
    if manifest.format_version != ARCHIVE_FORMAT_VERSION
        || manifest.schema_min != manifest.schema_max
        || manifest.record_count != expected.record_count
        || manifest.table_counts != expected.table_counts
        || manifest.table_hashes != expected.table_hashes
    {
        return Err(RepoError::InvalidBackup);
    }
    Ok(VerifiedArchive {
        envelope,
        manifest,
        archive_sha256,
    })
}

pub fn for_each_record(
    path: &Path,
    passphrase: &str,
    mut operation: impl FnMut(CanonicalRecord) -> Result<(), RepoError>,
) -> Result<ArchiveEnvelope, RepoError> {
    read_archive(path, passphrase, None, |item| match item {
        ArchiveItem::Record(record) => operation(record),
        ArchiveItem::Manifest(_) => Ok(()),
    })
}

pub fn for_each_record_with_cancellation(
    path: &Path,
    passphrase: &str,
    cancellation: &CancellationToken,
    mut operation: impl FnMut(CanonicalRecord) -> Result<(), RepoError>,
) -> Result<ArchiveEnvelope, RepoError> {
    read_archive(path, passphrase, Some(cancellation), |item| match item {
        ArchiveItem::Record(record) => operation(record),
        ArchiveItem::Manifest(_) => Ok(()),
    })
}

fn read_archive(
    path: &Path,
    passphrase: &str,
    cancellation: Option<&CancellationToken>,
    mut operation: impl FnMut(ArchiveItem) -> Result<(), RepoError>,
) -> Result<ArchiveEnvelope, RepoError> {
    check_cancellation(cancellation)?;
    let file = File::open(path).map_err(|_| RepoError::InvalidBackup)?;
    let mut input = BufReader::new(file);
    let mut magic = [0_u8; ARCHIVE_MAGIC.len()];
    input
        .read_exact(&mut magic)
        .map_err(|_| RepoError::InvalidBackup)?;
    if &magic != ARCHIVE_MAGIC {
        return Err(RepoError::InvalidBackup);
    }
    let envelope_length = read_u32(&mut input)? as usize;
    if envelope_length == 0 || envelope_length > MAX_ENVELOPE_BYTES {
        return Err(RepoError::InvalidBackup);
    }
    let mut envelope_bytes = vec![0_u8; envelope_length];
    input
        .read_exact(&mut envelope_bytes)
        .map_err(|_| RepoError::InvalidBackup)?;
    let mut header_checksum = [0_u8; HEADER_CHECKSUM_BYTES];
    input
        .read_exact(&mut header_checksum)
        .map_err(|_| RepoError::InvalidBackup)?;
    if header_checksum
        .ct_eq(Sha256::digest(&envelope_bytes).as_slice())
        .unwrap_u8()
        != 1
    {
        return Err(RepoError::InvalidBackup);
    }
    let envelope: ArchiveEnvelope =
        serde_json::from_slice(&envelope_bytes).map_err(|_| RepoError::InvalidBackup)?;
    envelope.validate()?;
    let key = derive_key(passphrase, &envelope.kdf)?;
    check_cancellation(cancellation)?;
    if key_check(&key[..])
        .ct_eq(&envelope.cipher.key_check)
        .unwrap_u8()
        != 1
    {
        return Err(RepoError::WrongPassphrase);
    }
    let cipher =
        ChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| RepoError::InvalidBackup)?;
    let envelope_digest: [u8; 32] = Sha256::digest(&envelope_bytes).into();
    let mut counter = 0_u64;
    let mut pending = Vec::new();
    let mut saw_final = false;

    loop {
        check_cancellation(cancellation)?;
        let mut flag = [0_u8; 1];
        match input.read_exact(&mut flag) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return Err(RepoError::InvalidBackup),
        }
        if saw_final || !matches!(flag[0], DATA_FRAME | FINAL_FRAME) {
            return Err(RepoError::InvalidBackup);
        }
        let ciphertext_length = read_u32(&mut input)? as usize;
        if !(TAG_BYTES..=STREAM_CHUNK_BYTES + TAG_BYTES).contains(&ciphertext_length) {
            return Err(RepoError::InvalidBackup);
        }
        let mut ciphertext = vec![0_u8; ciphertext_length];
        input
            .read_exact(&mut ciphertext)
            .map_err(|_| RepoError::InvalidBackup)?;
        let nonce_bytes = nonce_bytes(envelope.cipher.nonce_prefix, counter);
        let aad = frame_aad(envelope_digest, counter, flag[0]);
        let nonce =
            Nonce::try_from(nonce_bytes.as_slice()).map_err(|_| RepoError::InvalidBackup)?;
        let plaintext = cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| RepoError::InvalidBackup)?;
        pending.extend_from_slice(&plaintext);
        decode_lines(&mut pending, &mut operation)?;
        saw_final = flag[0] == FINAL_FRAME;
        counter = counter.checked_add(1).ok_or(RepoError::InvalidBackup)?;
    }
    if !saw_final || !pending.is_empty() {
        return Err(RepoError::InvalidBackup);
    }
    Ok(envelope)
}

fn check_cancellation(cancellation: Option<&CancellationToken>) -> Result<(), RepoError> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        Err(RepoError::InvalidTransition)
    } else {
        Ok(())
    }
}

fn decode_lines(
    pending: &mut Vec<u8>,
    operation: &mut impl FnMut(ArchiveItem) -> Result<(), RepoError>,
) -> Result<(), RepoError> {
    while let Some(position) = pending.iter().position(|byte| *byte == b'\n') {
        if position == 0 || position > MAX_RECORD_BYTES {
            return Err(RepoError::InvalidBackup);
        }
        let remainder = pending.split_off(position + 1);
        let line = std::mem::replace(pending, remainder);
        let item =
            serde_json::from_slice(&line[..position]).map_err(|_| RepoError::InvalidBackup)?;
        operation(item)?;
    }
    if pending.len() > MAX_RECORD_BYTES {
        return Err(RepoError::InvalidBackup);
    }
    Ok(())
}

fn derive_key(passphrase: &str, contract: &KdfContract) -> Result<Zeroizing<[u8; 32]>, RepoError> {
    if passphrase.is_empty() {
        return Err(RepoError::InvalidRequest);
    }
    let params = Params::new(
        contract.memory_kib,
        contract.iterations,
        contract.parallelism,
        Some(32),
    )
    .map_err(|_| RepoError::InvalidBackup)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; 32]);
    argon2
        .hash_password_into(passphrase.as_bytes(), &contract.salt, key.as_mut())
        .map_err(|_| RepoError::InvalidBackup)?;
    Ok(key)
}

fn nonce_bytes(prefix: [u8; 4], counter: u64) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    nonce[..4].copy_from_slice(&prefix);
    nonce[4..].copy_from_slice(&counter.to_be_bytes());
    nonce
}

fn key_check(key: &[u8]) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(b"mtgo-opponent-notes-archive-key-check-v1");
    hasher.update(key);
    let digest = hasher.finalize();
    let mut check = [0_u8; 16];
    check.copy_from_slice(&digest[..16]);
    check
}

fn frame_aad(envelope_digest: [u8; 32], counter: u64, flag: u8) -> [u8; 41] {
    let mut aad = [0_u8; 41];
    aad[..32].copy_from_slice(&envelope_digest);
    aad[32..40].copy_from_slice(&counter.to_be_bytes());
    aad[40] = flag;
    aad
}

fn read_u32(input: &mut impl Read) -> Result<u32, RepoError> {
    let mut bytes = [0_u8; 4];
    input
        .read_exact(&mut bytes)
        .map_err(|_| RepoError::InvalidBackup)?;
    Ok(u32::from_be_bytes(bytes))
}

fn hash_file(path: &Path) -> Result<String, RepoError> {
    let mut file = File::open(path).map_err(|_| RepoError::InvalidBackup)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; STREAM_CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| RepoError::InvalidBackup)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize()))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(table: &str, key: &str, text: &str) -> CanonicalRecord {
        CanonicalRecord {
            table: table.to_owned(),
            key: key.to_owned(),
            values: vec![CanonicalValue::Text(text.to_owned())],
        }
    }

    #[test]
    fn ut_077_manifest_contains_versions_counts_hashes_and_classifier_provenance() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("fixture.mtgonotes");
        let manifest = write_archive(
            &path,
            "correct horse battery staple",
            ArchiveEnvelope::with_material([7; 16], [9; 4]),
            [
                record("opponent_profiles", "a", "Alice"),
                record("observations", "b", "Keeps risky hands"),
            ],
            UtcMillis::new(1_700_000_000_000).expect("time"),
            1,
            vec![ClassifierProvenance {
                version: "2026.07.1".to_owned(),
                digest: "a".repeat(64),
            }],
        )
        .expect("archive");

        assert_eq!(manifest.format_version, 1);
        assert_eq!(manifest.schema_min, 1);
        assert_eq!(manifest.record_count, 2);
        assert_eq!(manifest.table_counts["observations"], 1);
        assert_eq!(manifest.table_hashes["observations"].len(), 64);
        assert_eq!(manifest.classifier_provenance.len(), 1);
        assert_eq!(
            verify_archive(&path, "correct horse battery staple")
                .expect("verify")
                .manifest,
            manifest
        );
    }

    #[test]
    fn wire_fixture_is_deterministic_and_contains_no_plaintext() {
        let first = tempfile::NamedTempFile::new().expect("file");
        let second = tempfile::NamedTempFile::new().expect("file");
        for path in [first.path(), second.path()] {
            write_archive(
                path,
                "fixture-passphrase",
                ArchiveEnvelope::with_material([1; 16], [2; 4]),
                [record("observations", "record-1", "CANARY-PLAINTEXT-NOTE")],
                UtcMillis::new(42).expect("time"),
                1,
                vec![],
            )
            .expect("archive");
        }
        let first_bytes = std::fs::read(first.path()).expect("read");
        assert_eq!(
            first_bytes,
            std::fs::read(second.path()).expect("read second")
        );
        assert!(
            !first_bytes
                .windows(b"CANARY-PLAINTEXT-NOTE".len())
                .any(|window| window == b"CANARY-PLAINTEXT-NOTE")
        );
    }

    #[test]
    fn ut_080_and_ut_081_authenticate_before_records_are_accepted() {
        let file = tempfile::NamedTempFile::new().expect("file");
        write_archive(
            file.path(),
            "right",
            ArchiveEnvelope::with_material([3; 16], [4; 4]),
            [record("opponent_profiles", "a", "Alice")],
            UtcMillis::new(42).expect("time"),
            1,
            vec![],
        )
        .expect("archive");

        assert!(verify_archive(file.path(), "right").is_ok());
        assert_eq!(
            verify_archive(file.path(), "wrong"),
            Err(RepoError::WrongPassphrase)
        );
        let mut damaged = std::fs::read(file.path()).expect("read");
        let last = damaged.len() - 1;
        damaged[last] ^= 1;
        std::fs::write(file.path(), damaged).expect("damage");
        assert_eq!(
            verify_archive(file.path(), "right"),
            Err(RepoError::InvalidBackup)
        );
    }

    #[test]
    fn ut_088_chunk_buffers_are_bounded_below_sixty_four_mib() {
        assert!(std::hint::black_box(STREAM_CHUNK_BYTES) < 64 * 1024 * 1024);
        assert!(std::hint::black_box(MAX_RECORD_BYTES) < 64 * 1024 * 1024);
    }
}
