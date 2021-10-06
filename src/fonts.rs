use std::cmp::Eq;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use thiserror::Error;

#[derive(Default)]
pub struct FontLoader {
    faces: HashMap<PathBuf, Result<Vec<u8>, FontLoadError>>,
}

#[derive(PartialEq, Eq)]
pub enum ForceLoad {
    True,
    False,
}

impl FontLoader {
    // TODO Until raw_entry_mut is stabilized, I have to clone pathbuf over and over again.
    pub fn get(
        &mut self,
        path: PathBuf,
        force: ForceLoad,
    ) -> Result<&[u8], &FontLoadError> {
        let res = match self.faces.entry(path) {
            Entry::Occupied(mut entry) => {
                if entry.get().is_err() && force == ForceLoad::True {
                    let _ = entry.insert(load_file_into_vec(entry.key()));
                }
                entry.into_mut()
            }
            Entry::Vacant(entry) => {
                let value = load_file_into_vec(entry.key());
                entry.insert(value)
            }
        };
        res.as_ref().map(|x| &x[..])
    }
}

fn load_file_into_vec(path: impl AsRef<Path>) -> Result<Vec<u8>, FontLoadError> {
    let mut v = Vec::new();
    BufReader::new(File::open(path)?).read_to_end(&mut v)?;
    Ok(v)
}

#[derive(Debug, Error)]
pub enum FontLoadError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
}
