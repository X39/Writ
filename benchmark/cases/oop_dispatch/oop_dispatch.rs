trait Computable {
    fn compute(&self) -> i64;
}

struct TypeA;
struct TypeB;
struct TypeC;
struct TypeD;

impl Computable for TypeA { fn compute(&self) -> i64 { 1 } }
impl Computable for TypeB { fn compute(&self) -> i64 { 2 } }
impl Computable for TypeC { fn compute(&self) -> i64 { 3 } }
impl Computable for TypeD { fn compute(&self) -> i64 { 4 } }

fn main() {
    let mut objects: Vec<Box<dyn Computable>> = Vec::with_capacity(100_000);
    for i in 0..100_000i64 {
        let obj: Box<dyn Computable> = match i % 4 {
            0 => Box::new(TypeA),
            1 => Box::new(TypeB),
            2 => Box::new(TypeC),
            3 => Box::new(TypeD),
            _ => unreachable!(),
        };
        objects.push(obj);
    }
    let sum: i64 = objects.iter().map(|o| o.compute()).sum();
    println!("{}", sum);
}
