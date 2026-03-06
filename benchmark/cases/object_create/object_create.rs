struct Point {
    x: i64,
    y: i64,
    label: &'static str,
}

fn main() {
    let mut sum: i64 = 0;
    for i in 0..1_000_000i64 {
        let p = Point { x: i, y: i, label: "item" };
        sum += p.x;
    }
    println!("{}", sum);
}
