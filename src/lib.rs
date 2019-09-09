use secp256k1::{
    constants::UNCOMPRESSED_PUBLIC_KEY_SIZE, Error as SecpError, PublicKey, SecretKey,
};

pub mod utils;

use utils::{aes_decrypt, aes_encrypt, decapsulate, encapsulate, generate_keypair};

pub fn encrypt(receiver_pub: &[u8], msg: &[u8]) -> Result<Vec<u8>, SecpError> {
    let receiver_pk = PublicKey::from_slice(receiver_pub)?;
    let (ephemeral_sk, ephemeral_pk) = generate_keypair();

    let aes_key = encapsulate(&ephemeral_sk, &receiver_pk);
    let encrypted = aes_encrypt(&aes_key, msg);

    let mut cipher_text = Vec::with_capacity(UNCOMPRESSED_PUBLIC_KEY_SIZE + encrypted.len());
    cipher_text.extend(ephemeral_pk.serialize_uncompressed().iter());
    cipher_text.extend(encrypted);

    Ok(cipher_text)
}

pub fn decrypt(receiver_sec: &[u8], msg: &[u8]) -> Result<Vec<u8>, SecpError> {
    let receiver_sk = SecretKey::from_slice(receiver_sec)?;

    let ephemeral_pk = PublicKey::from_slice(&msg[..UNCOMPRESSED_PUBLIC_KEY_SIZE])?;
    let encrypted = &msg[UNCOMPRESSED_PUBLIC_KEY_SIZE..];

    let aes_key = decapsulate(&ephemeral_pk, &receiver_sk);

    Ok(aes_decrypt(&aes_key, encrypted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::encode;
    use utils::{decode_hex, generate_keypair};

    const PYTHON_BACKEND: &str = "https://eciespy.herokuapp.com/";
    const MSG: &str = "helloworld";

    #[test]
    fn check_encrypt_decrypt() {
        let (sk, pk) = generate_keypair();
        let msg = MSG.as_bytes();
        assert_eq!(
            msg,
            decrypt(
                &sk[..],
                &encrypt(&pk.serialize_uncompressed(), msg).unwrap()
            )
            .unwrap()
            .as_slice()
        );
    }

    #[test]
    fn check_encrypt_decrypt_against_python() {
        let (sk, pk) = generate_keypair();

        let sk_hex = encode(&sk[..].to_vec());
        let uncompressed_pk = &pk.serialize_uncompressed();
        let pk_hex = encode(uncompressed_pk.to_vec());

        let client = reqwest::Client::new();
        let params = [("data", MSG), ("pub", pk_hex.as_str())];
        let res = client
            .post(PYTHON_BACKEND)
            .form(&params)
            .send()
            .unwrap()
            .text()
            .unwrap();

        let server_encrypted = decode_hex(&res);
        let local_decrypted = decrypt(&sk[..], server_encrypted.as_slice()).unwrap();
        assert_eq!(local_decrypted, MSG.as_bytes());

        let local_encrypted = encrypt(uncompressed_pk, MSG.as_bytes()).unwrap();
        let params = [("data", encode(local_encrypted)), ("prv", sk_hex)];

        let res = client
            .post(PYTHON_BACKEND)
            .form(&params)
            .send()
            .unwrap()
            .text()
            .unwrap();

        assert_eq!(res, MSG);
    }
}
