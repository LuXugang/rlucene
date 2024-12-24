use rand::Rng;

fn main() {
    let seed: u64 = rand::thread_rng().gen_range(0..u64::MAX);
    println!("{}", seed);
}
