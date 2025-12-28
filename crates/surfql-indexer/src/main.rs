use std::{collections::HashMap, sync::LazyLock};

use rust_stemmers::{Algorithm, Stemmer};
use unicode_segmentation::UnicodeSegmentation;

type TermFrequency = HashMap<String, f32>;
type InverseDocumentFrequency = HashMap<String, f32>;
type BM25 = HashMap<String, f32>;
type DocumentId = u32;
type Url = String;

static STEMMER: LazyLock<Stemmer> = LazyLock::new(|| Stemmer::create(Algorithm::English));

#[derive(Debug)]
pub struct Raw;
pub struct Stemmed {
    stems: Vec<String>,
}

#[derive(Debug)]
pub struct Document<S> {
    pub id: DocumentId,
    pub url: Url,
    pub content: String,
    state: S,
}

impl<T: Into<String>> From<T> for Document<Raw> {
    fn from(value: T) -> Self {
        Self {
            id: 0,
            url: String::new(),
            content: value.into(),
            state: Raw,
        }
    }
}

impl Document<Raw> {
    pub fn new(id: DocumentId, url: Url, content: impl Into<String>) -> Self {
        Self {
            id,
            url,
            content: content.into(),
            state: Raw,
        }
    }

    fn sanitize(&self) -> String {
        self.content.to_lowercase()
    }

    pub fn stem_tokens(&self) -> Vec<String> {
        self.sanitize()
            .unicode_words()
            .map(|token| STEMMER.stem(token).to_string())
            .collect::<Vec<String>>()
    }

    pub fn stem(self) -> Document<Stemmed> {
        let stems = self.stem_tokens();

        Document {
            id: self.id,
            url: self.url,
            content: self.content,
            state: Stemmed { stems },
        }
    }
}

impl Document<Stemmed> {
    /// Compute the term frequency component for the BM25 calculation of this term.
    pub fn compute_tf(self, k: f32, b: f32, average_doc_len: f32) -> TermFrequency {
        let doc_len = self.state.stems.len();
        let mut term_freq: HashMap<String, usize> = HashMap::new();
        let mut tf: TermFrequency = TermFrequency::new();

        self.state.stems.iter().for_each(|stem| {
            if term_freq.contains_key(stem) {
                *term_freq.get_mut(stem).unwrap() += 1;
            } else {
                term_freq.insert(stem.to_string(), 1);
            }
        });

        term_freq.into_iter().for_each(|(key, value)| {
            let doc_len = doc_len as f32;
            let computed_tf = (value as f32 * (k + 1.0))
                / (doc_len + (k * (1.0 - b * (b * (doc_len / average_doc_len)))));
            tf.insert(key, computed_tf);
        });

        tf
    }
}

pub struct Indexer {
    pub documents: Vec<Document<Raw>>,
}

impl Indexer {
    pub fn new(documents: Vec<Document<Raw>>) -> Self {
        Self { documents }
    }

    fn get_average_doc_len(&self) -> f32 {
        let sum: usize = self.documents.iter().map(|doc| doc.content.len()).sum();
        sum as f32 / self.documents.len() as f32
    }

    fn get_idf(&self) -> InverseDocumentFrequency {
        let mut count_docs: HashMap<String, usize> = HashMap::new();
        let mut idf = InverseDocumentFrequency::new();
        self.documents.iter().for_each(|doc| {
            let stem = doc.stem_tokens();
            stem.into_iter().for_each(|term| {
                count_docs
                    .entry(term)
                    .and_modify(|num| *num += 1)
                    .or_insert(1);
            });
        });

        let total_docs = self.documents.len() as f32;
        count_docs.into_iter().for_each(|(term, docs_count)| {
            let computed_idf = {
                let docs_count = docs_count as f32;
                ((total_docs - docs_count + 0.5_f32) / (docs_count + 0.5_f32)) + 1.0_f32
            };

            idf.insert(term, computed_idf);
        });

        idf
    }

    pub fn index(self) {
        let avedl = self.get_average_doc_len();
        let k = 1.25_f32;
        let b = 0.75_f32;
        let idf = self.get_idf();

        self.documents.into_iter().for_each(|doc| {
            let url = doc.url.clone();
            let tf = doc.stem().compute_tf(k, b, avedl);
            let bm25 = tf.into_iter().map(|(key, value)| {
                let value = value * *idf.get(&key).unwrap();
                (key, value)
            });
            println!("BM25 for {url}: {bm25:?}");
        });
    }
}

fn main() {
    println!("Hello World!");
}
