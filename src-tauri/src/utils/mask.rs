/// 脱敏 API Key：空串原样；长度 ≤ 8 返回 `****`；否则保留首 4 + 尾 4，中间以星号填充。
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = key.chars().collect();
    let len = chars.len();
    if len <= 8 {
        return "****".to_string();
    }
    let first: String = chars[..4].iter().collect();
    let last: String = chars[len - 4..].iter().collect();
    format!("{first}{}{last}", "*".repeat(len - 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_correctly() {
        assert_eq!(mask_api_key(""), "");
        assert_eq!(mask_api_key("short"), "****");
        assert_eq!(mask_api_key("12345678"), "****");
        assert_eq!(mask_api_key("0123456789ab"), "0123****89ab");
    }
}
