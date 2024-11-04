mod PriorityQueue;

use std::fmt::{Display, Formatter};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let a = 3;
        let b = 4;
        let result = add(a, b);
        println!("left is {} {}", a, b)
    }
}

pub fn add(left: usize, right: usize) -> () {
    println!("left is {} {}", left, right)
}

struct Abc {
    a: usize,
}

impl Display for Abc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.a)
    }
}

impl Abc {
    fn new(a: usize) -> Abc {
        Abc { a }
    }
}
