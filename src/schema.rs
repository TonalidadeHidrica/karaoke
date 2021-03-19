// use std::borrow::Borrow;
use std::iter;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;

use derive_more::From;
use druid::im::OrdMap;
use druid::im::Vector;
use druid::Data;
// use itertools::Itertools;
use druid::im::vector;
use num::rational::BigRational;
use num::One;
use num::Zero;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, From, Debug, Data)]
pub struct BeatPosition(#[data(same_fn = "PartialEq::eq")] pub BigRational);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, From, Debug, Data)]
pub struct BeatLength(#[data(same_fn = "PartialEq::eq")] pub BigRational);

impl BeatPosition {
    pub fn zero() -> Self {
        Self(BigRational::zero())
    }
}

impl BeatLength {
    pub fn one() -> Self {
        Self(BigRational::one())
    }

    pub fn four() -> Self {
        Self(BigRational::from_integer(4.into()))
    }
}

impl Add<BeatLength> for BeatPosition {
    type Output = BeatPosition;

    fn add(self, rhs: BeatLength) -> Self::Output {
        (self.0 + rhs.0).into()
    }
}

impl Add<&BeatLength> for &BeatPosition {
    type Output = BeatPosition;

    fn add(self, rhs: &BeatLength) -> Self::Output {
        self.clone() + rhs.clone()
    }
}

impl AddAssign<BeatLength> for BeatPosition {
    fn add_assign(&mut self, rhs: BeatLength) {
        self.0 += rhs.0
    }
}

impl AddAssign<&BeatLength> for BeatPosition {
    fn add_assign(&mut self, rhs: &BeatLength) {
        self.0 += rhs.0.clone()
    }
}

impl Sub<BeatLength> for BeatPosition {
    type Output = BeatPosition;

    fn sub(self, rhs: BeatLength) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

impl Sub<&BeatLength> for &BeatPosition {
    type Output = BeatPosition;

    fn sub(self, rhs: &BeatLength) -> Self::Output {
        self.clone() - rhs.clone()
    }
}

impl SubAssign<BeatLength> for BeatPosition {
    fn sub_assign(&mut self, rhs: BeatLength) {
        self.0 -= rhs.0
    }
}

impl SubAssign<&BeatLength> for BeatPosition {
    fn sub_assign(&mut self, rhs: &BeatLength) {
        self.0 -= rhs.0.clone()
    }
}

impl Sub<BeatPosition> for BeatPosition {
    type Output = BeatLength;

    fn sub(self, rhs: BeatPosition) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

impl Sub<&BeatPosition> for &BeatPosition {
    type Output = BeatLength;

    fn sub(self, rhs: &BeatPosition) -> Self::Output {
        self.clone() - rhs.clone()
    }
}

#[derive(Clone, Default, Data)]
pub struct Score {
    pub tracks: Vector<Track>,
    pub measure_lengths: OrdMap<BeatPosition, BeatLength>,
}

impl Score {
    pub fn sample_score() -> Self {
        let elements = "
            .....|||
            |.||..||
            ....<...
        "
        .chars()
        .filter_map(|c| {
            let kind = match c {
                '.' => ScoreElementKind::Skip,
                '|' => ScoreElementKind::Start,
                '<' => ScoreElementKind::Stop,
                _ => return None,
            };
            let length = BeatLength::from(BigRational::new(1.into(), 4.into()));
            Some(ScoreElement { kind, length })
        })
        .collect();

        let track = Track {
            start_beat: BeatPosition::from(BigRational::from_integer(44.into())),
            elements,
        };
        Score {
            tracks: vector![track],
            measure_lengths: OrdMap::new(),
        }
    }
}

#[derive(Clone, Data)]
pub struct Track {
    pub start_beat: BeatPosition,
    pub elements: Vector<ScoreElement>,
}

#[derive(Clone, PartialEq, Data)]
pub struct ScoreElement {
    pub kind: ScoreElementKind,
    pub length: BeatLength,
}

#[derive(Clone, PartialEq, Data)]
pub enum ScoreElementKind {
    Start,
    Stop,
    Skip,
}

// pub fn iterate_elements<'a>(
//     elements: impl Iterator<Item = impl Borrow<ScoreElement>> + 'a,
// ) -> impl Iterator<Item = (usize, usize)> + 'a {
//     use ScoreElementKind::*;
//     let mut elements = elements.enumerate().peekable();
//     iter::from_fn(move || {
//         let (i, _) = elements.find(|(_, e)| matches!(e.borrow().kind, Start))?;
//         let j = match elements
//             .peeking_take_while(|(_, e)| matches!(e.borrow().kind, Continued))
//             .last()
//         {
//             Some((k, _)) => k + 1,
//             None => i + 1,
//         };
//         Some((i, j))
//     })
// }

pub fn iterate_measures<'a>(
    measures: &'a OrdMap<BeatPosition, BeatLength>,
) -> impl Iterator<Item = (BeatPosition, BeatPosition)> + 'a {
    let mut measure_lengths = measures.iter().peekable();
    let mut measure_length = BeatLength::four();
    let mut measure_start_beat = BeatPosition::zero();

    iter::from_fn(move || {
        let mut measure_end_beat = &measure_start_beat + &measure_length;
        if let Some((next_measure_beat, next_measure_length)) = measure_lengths.peek() {
            if next_measure_beat == &&measure_start_beat {
                measure_length = next_measure_length.clone().to_owned();
                measure_end_beat = &measure_start_beat + &measure_length;
                measure_lengths.next();
            } else if next_measure_beat < &&measure_end_beat {
                measure_length = next_measure_length.clone().to_owned();
                measure_end_beat = next_measure_beat.clone().to_owned();
                measure_lengths.next();
            }
        }
        let ret = Some((measure_start_beat.clone(), measure_end_beat.clone()));
        measure_start_beat = measure_end_beat;
        ret
    })
}

#[cfg(test)]
mod test {
    // use super::iterate_elements;
    use super::iterate_measures;
    use super::BeatLength;
    use super::BeatPosition;
    // use super::ScoreElement;
    // use super::ScoreElementKind;
    use druid::im::ordmap;
    use itertools::iterate;
    use itertools::Itertools;
    use num::BigRational;

    // #[test]
    // fn test_iterate_elements() {
    //     use ScoreElementKind::*;
    //     let elements = vec![
    //         Start, Continued, Continued, Start, Continued, Continued, Empty, Empty, Start, Empty,
    //         Start, Empty,
    //     ]
    //     .into_iter()
    //     .map(|kind| ScoreElement { kind });
    //     assert_eq!(
    //         iterate_elements(elements).collect_vec(),
    //         vec![(0, 3), (3, 6), (8, 9), (10, 11),]
    //     );
    // }

    #[test]
    fn test_iterate_measures_01() {
        let measures = ordmap![];
        let got = iterate_measures(&measures).take(10).collect_vec();
        let expected = iterate(0, |x| x + 4)
            .map(|x| BeatPosition::from(BigRational::from_integer(x.into())))
            .tuple_windows::<(_, _)>()
            .take(10)
            .collect_vec();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_iterate_measures_02() {
        let measures = ordmap![
            BeatPosition::from(BigRational::from_integer(16.into())) => BeatLength::from(BigRational::from_integer(3.into()))
        ];
        let got = iterate_measures(&measures).take(10).collect_vec();
        let expected = vec![0, 4, 8, 12, 16, 19, 22, 25, 28, 31, 34]
            .into_iter()
            .map(|x| BeatPosition::from(BigRational::from_integer(x.into())))
            .tuple_windows::<(_, _)>()
            .collect_vec();
        assert_eq!(got, expected);
    }
}
