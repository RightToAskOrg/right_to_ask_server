pub mod person;
pub mod regions;
pub mod database;
pub mod signing;
pub mod config;
pub mod mp;
mod parse_mp_lists;
mod parse_pdf_util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
