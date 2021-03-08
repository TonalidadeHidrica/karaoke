use std::borrow::Borrow;

use derive_more::From;
use druid::im::OrdMap;
use druid::im::Vector;
use druid::Data;
use itertools::Itertools;
use num::rational::BigRational;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Data, From)]
pub struct BeatPosition(#[data(same_fn = "PartialEq::eq")] BigRational);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Data, From)]
pub struct BeatLength(#[data(same_fn = "PartialEq::eq")] BigRational);

#[derive(Clone, Default, Data)]
pub struct Score {
    pub elements: Vector<ScoreElement>,
    pub measure_lengths: OrdMap<BeatPosition, BeatLength>,
}

#[derive(Clone, PartialEq, Data)]
pub struct ScoreElement {
    pub kind: ScoreElementKind,
}

#[derive(Clone, PartialEq, Data)]
pub enum ScoreElementKind {
    Start,
    Continued,
    Empty,
}

pub fn iterate_elements<'a>(
    elements: impl Iterator<Item = impl Borrow<ScoreElement>> + 'a,
) -> impl Iterator<Item = (usize, usize)> + 'a {
    use ScoreElementKind::*;
    let mut elements = elements.enumerate().peekable();
    std::iter::from_fn(move || {
        let (i, _) = elements.find(|(_, e)| matches!(e.borrow().kind, Start))?;
        let j = match elements
            .peeking_take_while(|(_, e)| matches!(e.borrow().kind, Continued))
            .last()
        {
            Some((k, _)) => k + 1,
            None => i + 1,
        };
        Some((i, j))
    })
}

#[cfg(test)]
mod test {
    use super::{iterate_elements, ScoreElement, ScoreElementKind};
    use itertools::Itertools;

    #[test]
    fn test_iterate_elements() {
        use ScoreElementKind::*;
        let elements = vec![
            Start, Continued, Continued, Start, Continued, Continued, Empty, Empty, Start, Empty,
            Start, Empty,
        ]
        .into_iter()
        .map(|kind| ScoreElement { kind });
        assert_eq!(
            iterate_elements(elements).collect_vec(),
            vec![(0, 3), (3, 6), (8, 9), (10, 11),]
        );
    }
}
