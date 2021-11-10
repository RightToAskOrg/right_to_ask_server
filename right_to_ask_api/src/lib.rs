pub mod person;
pub mod regions;
pub mod database;
pub mod signing;
pub mod config;
pub mod mp;
mod parse_mp_lists;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
