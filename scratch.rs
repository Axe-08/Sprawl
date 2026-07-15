fn main() {
    let s = "AKIAIOSFODNN7EXAMPLE";
    let mut freq = [0u32; 256];
    for byte in s.bytes() { freq[byte as usize] += 1; }
    let len = s.len() as f64;
    let ent = freq.iter().filter(|&&c| c > 0).map(|&c| { let p = c as f64 / len; -p * p.log2() }).sum::<f64>();
    println!("entropy: {}", ent);
}
