use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    for i in 0..100_000i64 {
        map.insert(format!("key_{}", i), i);
    }
    let mut sum: i64 = 0;
    for i in 0..100_000i64 {
        sum += map[&format!("key_{}", i)];
    }
    println!("{}", sum);
}
