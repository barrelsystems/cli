use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha2::{Digest, Sha512};
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

/// Streaming read buffer size, so memory use stays flat regardless of
/// artifact size instead of scaling with it.
const CHUNK_SIZE: usize = 1024 * 1024;

/// The integrity algorithm name Barrel's release-creation API expects.
pub const ALGORITHM: &str = "sha512";

/// SHA-512 checksum of a file's bytes, base64-encoded (Barrel's API expects
/// base64, not hex). Must be called on the artifact's final bytes -- signing
/// commonly rewrites them, so a pre-signing checksum won't match.
pub fn sha512_base64(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buffer = vec![0u8; CHUNK_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(BASE64.encode(hasher.finalize()))
}

pub fn file_size(path: &Path) -> io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha512_base64_matches_known_vector() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"").unwrap();

        // SHA-512 of the empty string, base64-encoded -- a standard,
        // independently-verifiable test vector (NIST/RFC test input).
        let digest = sha512_base64(file.path()).unwrap();
        assert_eq!(
            digest,
            "z4PhNX7vuL3xVChQ1m2AB9Yg5AULVxXcg/SpIdNs6c5H0NE8XYXysP+DGNKHfuwvY7kxvUdBeoGlODJ6+SfaPg=="
        );
    }

    #[test]
    fn file_size_matches_actual_bytes_written() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"twelve bytes").unwrap();
        assert_eq!(file_size(file.path()).unwrap(), 12);
    }
}
