use std::fs::File;
use std::path::PathBuf;
use std::io::copy;

use sha2::{Digest, Sha256};

use fluvio_hub_protocol::{Result};
use fluvio_hub_protocol::constants::HUB_PACKAGE_EXT;

/// non validating function to make canonical filenames from
/// org pkg version triples
pub fn make_filename(org: &str, pkg: &str, ver: &str) -> String {
    if org.is_empty() {
        format!("{pkg}-{ver}.{HUB_PACKAGE_EXT}")
    } else {
        format!("{org}-{pkg}-{ver}.{HUB_PACKAGE_EXT}")
    }
}

/// Generates Sha256 checksum for a given file
pub fn sha256_digest(path: &PathBuf) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path)?;

    copy(&mut file, &mut hasher)?;

    let hash_bytes = hasher.finalize();

    Ok(hex::encode(hash_bytes))
}

#[cfg(test)]
mod util_tests {
    use tempfile::TempDir;

    use crate::sha256_digest;

    #[test]
    fn creates_shasum_digest() {
        use std::fs::write;

        let tempdir = TempDir::new().unwrap();
        let temp_dir_path = tempdir.into_path().to_path_buf();
        let foo_path = temp_dir_path.join("foo");

        write(&foo_path, "foo").unwrap();

        let foo_a_checksum = sha256_digest(&foo_path).unwrap();

        assert_eq!(
            foo_a_checksum,
            "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
        );
    }

    #[test]
    fn checks_files_checksums_diff() {
        use std::fs::write;

        let tempdir = TempDir::new().unwrap();
        let temp_dir_path = tempdir.into_path().to_path_buf();
        let foo_path = temp_dir_path.join("foo");
        let bar_path = temp_dir_path.join("bar");

        write(&foo_path, "foo").unwrap();
        write(&bar_path, "bar").unwrap();

        let foo_checksum = sha256_digest(&foo_path).unwrap();
        let bar_checksum = sha256_digest(&bar_path).unwrap();

        assert_ne!(foo_checksum, bar_checksum);
    }

    #[test]
    fn checks_files_checksums_same() {
        use std::fs::write;

        let tempdir = TempDir::new().unwrap();
        let temp_dir_path = tempdir.into_path().to_path_buf();
        let foo_path = temp_dir_path.join("foo");

        write(&foo_path, "foo").unwrap();

        let foo_a_checksum = sha256_digest(&foo_path).unwrap();
        let foo_b_checksum = sha256_digest(&foo_path).unwrap();

        assert_eq!(foo_a_checksum, foo_b_checksum);
    }
}
