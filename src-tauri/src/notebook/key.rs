use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::RepoError;

pub const DATABASE_KEY_BYTES: usize = 32;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DatabaseKey([u8; DATABASE_KEY_BYTES]);

impl DatabaseKey {
    pub fn generate() -> Result<Self, RepoError> {
        let mut bytes = [0_u8; DATABASE_KEY_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| RepoError::KeyUnavailable)?;
        Ok(Self(bytes))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RepoError> {
        let bytes: [u8; DATABASE_KEY_BYTES] =
            bytes.try_into().map_err(|_| RepoError::KeyUnavailable)?;
        Ok(Self(bytes))
    }

    pub fn expose(&self) -> &[u8; DATABASE_KEY_BYTES] {
        &self.0
    }
}

pub trait KeyProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError>;
    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError>;
}

#[derive(Clone, Copy, Default)]
pub struct CurrentUserDpapi;

pub struct KeyCustody<P> {
    sealed_key_path: PathBuf,
    database_path: PathBuf,
    protector: P,
}

impl<P: KeyProtector> KeyCustody<P> {
    pub fn new(
        sealed_key_path: impl Into<PathBuf>,
        database_path: impl Into<PathBuf>,
        protector: P,
    ) -> Self {
        Self {
            sealed_key_path: sealed_key_path.into(),
            database_path: database_path.into(),
            protector,
        }
    }

    pub fn load_or_create(&self) -> Result<DatabaseKey, RepoError> {
        if self.sealed_key_path.exists() {
            let sealed = fs::read(&self.sealed_key_path).map_err(|_| RepoError::KeyUnavailable)?;
            let mut plaintext = self.protector.unprotect(&sealed)?;
            let key = DatabaseKey::from_bytes(&plaintext);
            plaintext.zeroize();
            return key;
        }

        if self.database_path.exists() {
            return Err(RepoError::KeyUnavailable);
        }

        let key = DatabaseKey::generate()?;
        let sealed = self.protector.protect(key.expose())?;
        write_atomic_private(&self.sealed_key_path, &sealed)?;
        Ok(key)
    }
}

fn write_atomic_private(path: &Path, contents: &[u8]) -> Result<(), RepoError> {
    let parent = path.parent().ok_or(RepoError::KeyUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| RepoError::KeyUnavailable)?;
    let partial = path.with_extension("partial");
    if partial.exists() {
        fs::remove_file(&partial).map_err(|_| RepoError::KeyUnavailable)?;
    }
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&partial)
        .map_err(|_| RepoError::KeyUnavailable)?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|_| RepoError::KeyUnavailable)?;
    fs::rename(&partial, path).map_err(|_| RepoError::KeyUnavailable)?;
    Ok(())
}

#[cfg(windows)]
impl KeyProtector for CurrentUserDpapi {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        use windows::Win32::Foundation::{HLOCAL, LocalFree};
        use windows::Win32::Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
        };
        use windows::core::PCWSTR;

        let input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(plaintext.len()).map_err(|_| RepoError::KeyUnavailable)?,
            pbData: plaintext.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &input,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .map_err(|_| RepoError::KeyUnavailable)?;
            let protected =
                std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
            Ok(protected)
        }
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        use windows::Win32::Foundation::{HLOCAL, LocalFree};
        use windows::Win32::Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
        };

        let input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(ciphertext.len()).map_err(|_| RepoError::KeyUnavailable)?,
            pbData: ciphertext.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .map_err(|_| RepoError::KeyUnavailable)?;
            let plaintext =
                std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
            Ok(plaintext)
        }
    }
}

#[cfg(not(windows))]
impl KeyProtector for CurrentUserDpapi {
    fn protect(&self, _plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Err(RepoError::KeyUnavailable)
    }

    fn unprotect(&self, _ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Err(RepoError::KeyUnavailable)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[derive(Clone)]
    pub(crate) struct ScopedProtector {
        pub(crate) scope: u8,
        pub(crate) fail_unprotect: bool,
    }

    impl KeyProtector for ScopedProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
            let mut protected = Vec::with_capacity(plaintext.len() + 1);
            protected.push(self.scope);
            protected.extend(plaintext.iter().map(|byte| byte ^ self.scope));
            Ok(protected)
        }

        fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
            if self.fail_unprotect || ciphertext.first() != Some(&self.scope) {
                return Err(RepoError::KeyUnavailable);
            }
            Ok(ciphertext[1..]
                .iter()
                .map(|byte| byte ^ self.scope)
                .collect())
        }
    }

    #[test]
    fn ut_032_unseal_failure_does_not_replace_database_or_key() {
        let directory = tempfile::tempdir().expect("tempdir");
        let database_path = directory.path().join("notebook.db");
        let key_path = directory.path().join("notebook.key");
        fs::write(&database_path, b"existing-database").expect("database");
        fs::write(&key_path, b"existing-key").expect("key");
        let custody = KeyCustody::new(
            &key_path,
            &database_path,
            ScopedProtector {
                scope: 7,
                fail_unprotect: true,
            },
        );

        assert!(matches!(
            custody.load_or_create(),
            Err(RepoError::KeyUnavailable)
        ));
        assert_eq!(
            fs::read(&database_path).expect("database"),
            b"existing-database"
        );
        assert_eq!(fs::read(&key_path).expect("key"), b"existing-key");
    }

    #[test]
    fn it_278_current_user_scope_round_trips_and_foreign_scope_fails() {
        let directory = tempfile::tempdir().expect("tempdir");
        let database_path = directory.path().join("notebook.db");
        let key_path = directory.path().join("notebook.key");
        let owner = KeyCustody::new(
            &key_path,
            &database_path,
            ScopedProtector {
                scope: 7,
                fail_unprotect: false,
            },
        );
        let original = owner.load_or_create().expect("owner key");
        let reopened = owner.load_or_create().expect("same user key");
        assert_eq!(original.expose(), reopened.expose());

        let foreign_user = KeyCustody::new(
            &key_path,
            &database_path,
            ScopedProtector {
                scope: 8,
                fail_unprotect: false,
            },
        );
        assert!(matches!(
            foreign_user.load_or_create(),
            Err(RepoError::KeyUnavailable)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_dpapi_round_trip_uses_current_user() {
        let key = DatabaseKey::generate().expect("key");
        let sealed = CurrentUserDpapi.protect(key.expose()).expect("seal");
        let opened = CurrentUserDpapi.unprotect(&sealed).expect("unseal");
        assert_eq!(opened, key.expose());
    }
}
