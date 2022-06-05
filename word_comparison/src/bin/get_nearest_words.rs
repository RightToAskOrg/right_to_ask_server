//! Get words closest to other words.
//! Use to create (once) a synonym list.

use word_comparison::word::{read_glove, WordSource};
use word_comparison::near_words::{WordAndValue, SmallestN, print_near_words_vec};
use word_comparison::word_file::{write_word_file, WordsInFile, WORD_MMAP_FILE};


fn bad_args() { println!("Arguments should be `create <source_path>' or `test' (or old)");}

fn main() -> std::io::Result<()>{
    let args: Vec<String> = std::env::args().collect();
    if args.len()>1 {
        match args[1].as_str() {
            "create" => {
                let path = if args.len()>2 { args[2].as_str() } else { bad_args(); return Ok(())};
                let (words,wordvecs) = read_glove(path,None)?;
                write_word_file(WORD_MMAP_FILE,&words,&wordvecs,20)?;
            }
            "test" => { check_word_file()?; }
            "old" => { print_text()?; }
            _ => bad_args()
        }
    } else { bad_args() }
    Ok(())
}

fn check_word_file() -> std::io::Result<()>{
    let words = WordsInFile::read_word_file(WORD_MMAP_FILE)?;
    for i in words.all_indices() {
        let word = words.word(i);
        println!("{}\t{}",word,print_near_words_vec(&words,&words.synonyms(i),false));
        let lookup = words.index(word);
        assert_eq!(lookup,Some(i));
    }
    Ok(())
}

/// other thing that could be done.
#[allow(dead_code)]
fn print_text() -> std::io::Result<()>{
    let (words,wordvecs) = read_glove("/big/shared/NLP.glove/glove6B/glove.6B.50d.txt",None)?;
    for i in words.all_indices() {
        let word = words.word(i);
        let vec_i = wordvecs.get(i);
        let mut best_dot = SmallestN::new(10);
        let mut best_cosine = SmallestN::new(10);
        let mut best_distance = SmallestN::new(10);
        for j in words.all_indices() {
            let vec_j = wordvecs.get(j);
            best_dot.add(WordAndValue{ word: j, value: -vec_i.dot_product(vec_j) as f32 });
            best_cosine.add(WordAndValue{ word: j, value: -vec_i.cosine(vec_j) as f32 });
            best_distance.add(WordAndValue{ word: j, value: vec_i.distance(vec_j) as f32 })
        }
        let best_dot = best_dot.into_sorted_vec();
        let best_cosine = best_cosine.into_sorted_vec();
        let best_distance = best_distance.into_sorted_vec();
        println!("Word {}",word);
        println!("Dot{}",print_near_words_vec(&words,&best_dot,true));
        println!("Cos{}",print_near_words_vec(&words,&best_cosine,true));
        println!("D{}",print_near_words_vec(&words,&best_distance,false));
        println!();
    }

    Ok(())
}