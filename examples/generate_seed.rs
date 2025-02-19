use rand::Rng;

fn main() {
    let seed: u64 = rand::rng().random_range(0..u64::MAX);
    println!("{}", seed);
}
