// Test file for CodeRail markers

// @c#r simplify this function
fn calculate_total(items: Vec<i32>) -> i32 {
    let mut total = 0;
    for i in 0..items.len() {
        let item = items[i];
        if item > 0 {
            total = total + item;
        } else {
            // ignore negative
        }
    }
    return total;
}

// @c#t
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// @c#d
pub struct Config {
    pub name: String,
    pub value: i32,
    pub enabled: bool,
}

fn main() {
    let nums = vec![1, 2, 3, -1, 5];
    let total = calculate_total(nums); // @c#r.b.l inline: use iterator
    println!("Total: {}", total);
}
