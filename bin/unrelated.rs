
pub fn main() {
    println!("{}", column("ZY".to_owned()));
    println!("{}", int_to_column(701));
}

pub fn column(column_title: String) -> i32 {
    column_title.chars().rev().enumerate().fold(0, |acc, (idx, c) | acc + ((c as u32 - 64) * (26_u32).pow(idx as u32)) as i32)
}

pub fn int_to_column(i: i32) -> i32 {
    let k = i % 26;
    k
}