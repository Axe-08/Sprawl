pub fn shannon_entropy(s: &str) -> f64 {
    let mut freq = [0u32; 256];
    for byte in s.bytes() {
        freq[byte as usize] += 1;
    }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&count| count > 0)
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_of_repeated_character_is_zero() {
        assert_eq!(shannon_entropy("aaaaaaaaaaaaaaaa"), 0.0);
    }

    #[test]
    fn test_entropy_of_high_entropy_string() {
        // A truly random base64 string should have an entropy close to 6.0
        let s = "Vq1B9xLz4M6nPw3Xm0R8bQv7Kj2YcF5t"; 
        assert!(shannon_entropy(s) > 4.5);
    }
}
