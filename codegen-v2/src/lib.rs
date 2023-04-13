#[macro_use]
extern crate serde;

use grammar::{GHeaderFileItem, ParseTree};
use reader::Reader;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

mod grammar;
mod reader;
#[cfg(test)]
mod tests;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Todo,
    Eof,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CHeaderDirectory {
    map: HashMap<PathBuf, Vec<GHeaderFileItem>>,
}

pub fn parse_dir(path: &Path, mut main_dir: CHeaderDirectory) -> Result<CHeaderDirectory> {
    let entries = std::fs::read_dir(path).unwrap();
    for entry in entries {
        let child_path = entry.unwrap().path();

        if child_path.is_dir() {
            main_dir = parse_dir(&child_path, main_dir)?;
        } else if child_path.is_file() {
            main_dir = parse_file(&child_path, main_dir)?;
        } else {
            panic!()
        }
    }

    Ok(main_dir)
}

pub fn parse_file(path: &Path, mut main_dir: CHeaderDirectory) -> Result<CHeaderDirectory> {
    if !path.is_file() {
        return Err(Error::Todo);
    }

    let parent = path.parent().unwrap();

    let content = std::fs::read_to_string(path).map_err(|_err| Error::Todo)?;
    // TODO: Find a better solution for this.
    let content: String = content.chars().filter(|c| c.is_ascii()).collect();

    let mut reader = Reader::from(content.as_str());
    let mut items = vec![];

    loop {
        let (pending, checked_out) = reader.checkout();
        if let Ok(der) = GHeaderFileItem::derive(checked_out) {
            let (derived, branch) = (der.derived, der.branch);
            reader = pending.merge(branch);

            if let GHeaderFileItem::HeaderInclude(include) = &derived {
                let child_path = parent.join(Path::new(&include.0));

                // Skip referencing includes.
                if child_path == path {
                    continue;
                }

                // If the `#include`d file is not yet parsed, parse it.
                // Otherwise, skip it.
                if !main_dir.map.contains_key(&child_path) {
                    main_dir = parse_file(&child_path, main_dir)?;
                }
            }

            // Indicates EOF, implying this loop must exit.
            if let GHeaderFileItem::Eof = derived {
                break;
            }

            items.push(derived);
        } else {
            reader = pending.discard();
        }
    }

    // Update the map with newly parsed items.
    main_dir.map.insert(path.to_path_buf(), items);

    Ok(main_dir)
}

#[test]
#[ignore]
fn test_parse_file() {
    let path = Path::new("../include/");

    let dir = CHeaderDirectory {
        map: HashMap::new(),
    };

    let dir = parse_dir(path, dir).unwrap();
    //let string = serde_json::to_string_pretty(&dir).unwrap();

    println!("UNRECOGNIZED items:");

    for (path, items) in dir.map {
        println!("## FILE: {}", path.to_str().unwrap());
        for item in items {
            if let GHeaderFileItem::Unrecognized(item) = item {
                println!(">  {}", item);
            }
        }
    }
}

#[macro_export]
macro_rules! define_char {
    ($name:ident, $char:expr) => {
        #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
        pub struct $name;

        impl ParseTree for $name {
            type Derivation = Self;

            fn derive(reader: Reader<'_>) -> Result<DerivationResult<'_, Self::Derivation>> {
                let (slice, handle) = reader.read_amt(1)?;

                if let Some(symbol) = slice {
                    if symbol == $char {
                        return Ok(DerivationResult {
                            derived: $name,
                            branch: handle.commit().into_branch(),
                        });
                    }
                }

                Err(Error::Todo)
            }
        }
    };
}
