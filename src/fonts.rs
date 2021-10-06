use std::cmp::Eq;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufReader, Read};
use std::path::Path;
use std::{borrow::Borrow, collections::HashMap, path::PathBuf};

use thiserror::Error;

#[derive(Default)]
pub struct FontLoader {
    faces: HashMap<PathBuf, Result<Vec<u8>, FontLoadError>>,
}

pub enum ForceLoad {
    True,
    False,
}

impl FontLoader {
    // TODO do I have to access the map three times?
    pub fn get<'a, Q>(
        &'a mut self,
        path: &Q,
        force: ForceLoad,
    ) -> Result<&'a [u8], &'a FontLoadError>
    where
        PathBuf: Borrow<Q>,
        Q: Hash + Eq + AsRef<Path>,
    {
        match (self.faces.get(path), force) {
            (Some(Ok(data)), _) => Ok(&data[..]),
            (Some(Err(err)), ForceLoad::False) => Err(err),
            (Some(Err(_)), ForceLoad::True) | (None, _) => {
                let loaded = load_file_into_vec(path.as_ref());
                self.faces.insert(path.as_ref().to_owned(), loaded);
                self.faces.get(path).unwrap().as_ref().map(|x| &x[..])
            }
        }
    }
}

fn load_file_into_vec(path: impl AsRef<Path>) -> Result<Vec<u8>, FontLoadError> {
    let mut v = Vec::new();
    BufReader::new(File::open(path)?).read_to_end(&mut v)?;
    Ok(v)
}

#[derive(Debug, Error)]
enum FontLoadError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
}
