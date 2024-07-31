use std::collections::HashSet;
use std::sync::RwLock;

pub struct MasterOfNumbers {
    numbers: RwLock<Vec<usize>>,
    known_numbers: RwLock<HashSet<usize>>,
}

impl MasterOfNumbers {
    pub fn new() -> Self {
        MasterOfNumbers {
            numbers: RwLock::new(vec![]),
            known_numbers: RwLock::new(HashSet::new()),
        }
    }

    pub fn add_number(&self, number: usize) -> bool {
        let mut known_numbers = self.known_numbers.write().unwrap();
        if known_numbers.contains(&number) {
            return false;
        }
        known_numbers.insert(number);

        let mut numbers = self.numbers.write().unwrap();
        numbers.push(number);
        true
    }

    pub fn extend_numbers(&self, update: &[usize]) -> (usize, usize) {
        let mut known_numbers = self.known_numbers.write().unwrap();
        let mut numbers = self.numbers.write().unwrap();

        for &n in update {
            if known_numbers.contains(&n) {
                continue;
            }
            numbers.push(n);
            known_numbers.insert(n);
        }
        (numbers.len(), numbers.iter().sum())
    }

    pub fn read_numbers(&self, last_n: Option<usize>) -> Vec<usize> {
        let numbers = self.numbers.read().unwrap();
        if let Some(n) = last_n {
            return numbers.iter().rev().take(n).cloned().collect();
        }
        numbers.clone()
    }

    pub fn get_sum_count(&self) -> (usize, usize) {
        let numbers = self.numbers.read().unwrap();
        (numbers.len(), numbers.iter().sum())
    }
}
