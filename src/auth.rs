pub fn generate_api_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token1 = generate_api_token();
        let token2 = generate_api_token();

        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        assert_ne!(token1, token2);
    }
}
