mod crawler;
mod indexer;

use std::{
    error::Error,
    fs::{read_dir, read_to_string},
};

use dom_content_extraction::{
    DensityTree, get_node_text,
    scraper::{Html, Selector},
};

use crate::indexer::{Document, Indexer, Raw};

fn main() -> Result<(), Box<dyn Error>> {
    if let Ok(entries) = read_dir("samples/") {
        let mut documents: Vec<Document<Raw>> = vec![];
        for (id, entry) in entries.enumerate() {
            let path = entry?.path();
            let raw_contents = read_to_string(path.clone())?;
            let contents = get_main_content(raw_contents)?;
            documents.push(Document::new(
                (id + 1_usize) as u32,
                path.to_str().unwrap().to_string(),
                contents,
            ));
        }

        dbg!(&documents);
        Indexer::new(documents).index();
    };

    Ok(())
}

fn get_main_content(contents: String) -> Result<String, Box<dyn Error>> {
    let document = Html::parse_document(&contents);
    let selector = Selector::parse("meta[name=description]")?;

    if let Some(element) = document.select(&selector).next() {
        return Ok(element.attr("content").unwrap().to_string());
    } else {
        let dtree = DensityTree::from_document(&document)?;
        let sorted_nodes = dtree.sorted_nodes();
        if let Some(node) = sorted_nodes.iter().rev().take(1).next() {
            return Ok(get_node_text(node.node_id, &document)?);
        }
    }

    Ok(String::new())
}
