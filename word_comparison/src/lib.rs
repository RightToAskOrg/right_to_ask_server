pub mod word;
pub mod word_file;
pub mod near_words;
pub mod sentences;
pub mod listed_keywords;
pub mod comparison_list;
pub mod database_backend;
pub mod flatfile_database_backend;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
