use word_comparison::word_file::{WordsInFile, WORD_MMAP_FILE};
use word_comparison::listed_keywords::ListedKeywords;
use word_comparison::sentences::TokenizedSentence;

fn main() -> std::io::Result<()>{
    let words = WordsInFile::read_word_file(WORD_MMAP_FILE)?;
    let keywords = ListedKeywords::load(ListedKeywords::STD_LOCATION)?;
    let sentence = "Was it 5G interference that caused my phone data to stop working after I had my second covid vaccine? Or was it ScoMo's cat's left ear?";
    println!("Parsing {}",sentence);
    let parsed = TokenizedSentence::tokenize(sentence, &words, &keywords);
    parsed.explain(&words,&keywords);

    println!("wood {:?}",words.index_starting("wood"));
    println!("wood? {:?}",words.index_starting("wood?"));
    println!("woodf {:?}",words.index_starting("woodf"));
    println!("wood f {:?}",words.index_starting("wood f"));
    Ok(())
}