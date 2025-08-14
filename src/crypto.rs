use anyhow::{anyhow, Result};

/// 解密数据
pub fn decrypt_data(encrypted_data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    use aes::cipher::block_padding::Pkcs7;
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use cbc::Decryptor;

    let cipher = Decryptor::<aes::Aes128>::new(key.into(), iv.into());
    let mut buf = encrypted_data.to_vec();
    let decrypted_slice = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| anyhow!("Decryption error: {}", e))?;

    Ok(decrypted_slice.to_vec())
}
