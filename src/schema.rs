use std::borrow::Borrow;
use std::cmp::Ordering::*;
use std::convert::Infallible;
use std::iter;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Mul;
use std::ops::Sub;
use std::ops::SubAssign;
use std::path::PathBuf;

use derive_more::From;
use derive_new::new;
use druid::im::OrdMap;
use druid::im::Vector;
use druid::Data;
use druid::Lens;
use itertools::Itertools;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use serde::{Deserialize, Serialize};

use crate::serde_ord_map;
use crate::serde_ord_map::DeserializeKey;
use crate::serde_ord_map::SerializeKey;

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    Debug,
    derive_more::Display,
    Serialize,
    Deserialize,
    Data,
)]
pub struct BeatPosition(#[data(eq)] pub BigRational);
#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    Debug,
    derive_more::Display,
    Serialize,
    Deserialize,
    Data,
)]
pub struct BeatLength(#[data(eq)] pub BigRational);

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

impl Mul<u64> for BeatLength {
    type Output = BeatLength;

    fn mul(self, rhs: u64) -> BeatLength {
        BeatLength(self.0 * BigInt::from(rhs))
    }
}

impl SerializeKey for BeatPosition {
    type Error = Infallible;
    fn serialize_key(&self) -> Result<String, Infallible> {
        self.0.serialize_key()
    }
}

impl DeserializeKey for BeatPosition {
    type Error = anyhow::Error;
    fn deserialize_key(s: &str) -> anyhow::Result<Self> {
        BigRational::deserialize_key(s).map(Self)
    }
}

#[derive(
    Clone,
    Debug,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Display,
    Serialize,
    Deserialize,
)]
pub struct BigIntData(BigInt);

impl Data for BigIntData {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug, derive_more::Display, Serialize, Deserialize, Data, Lens)]
#[display(fmt = "{}/{}", numerator, denominator)]
pub struct MeasureLength {
    numerator: BigIntData,
    denominator: BigIntData,
}

impl Default for MeasureLength {
    fn default() -> Self {
        Self::new(4, 4)
    }
}

impl From<BeatLength> for MeasureLength {
    fn from(b: BeatLength) -> Self {
        let (x, y) = b.0.into();
        Self {
            numerator: BigIntData(x),
            denominator: BigIntData(y * 4),
        }
    }
}

impl From<MeasureLength> for BeatLength {
    fn from(m: MeasureLength) -> Self {
        Self(BigRational::new(m.numerator.0 * 4, m.denominator.0))
    }
}

impl MeasureLength {
    pub fn new(numerator: impl Into<BigInt>, denominator: impl Into<BigInt>) -> Self {
        Self {
            numerator: BigIntData(numerator.into()),
            denominator: BigIntData(denominator.into()),
        }
    }

    pub fn four() -> Self {
        BeatLength::four().into()
    }
}

#[derive(
    Clone, Copy, Debug, derive_more::FromStr, derive_more::Display, Serialize, Deserialize, Data,
)]
pub struct Bpm(pub f64);

impl Default for Bpm {
    fn default() -> Self {
        Self(120.0)
    }
}

impl Bpm {
    fn beat_length(&self) -> f64 {
        60.0 / self.0
    }
}

impl Mul<Bpm> for &BeatLength {
    type Output = f64;

    fn mul(self, rhs: Bpm) -> Self::Output {
        self.0.to_f64().unwrap() * rhs.beat_length()
    }
}

impl Mul<Bpm> for BeatLength {
    type Output = f64;

    fn mul(self, rhs: Bpm) -> Self::Output {
        &self * rhs
    }
}

impl Mul<&BeatLength> for Bpm {
    type Output = f64;

    fn mul(self, rhs: &BeatLength) -> Self::Output {
        rhs * self
    }
}

impl Mul<BeatLength> for Bpm {
    type Output = f64;

    fn mul(self, rhs: BeatLength) -> Self::Output {
        rhs * self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, new, Data, Lens)]
pub struct Score {
    #[new(default)]
    pub tracks: Vector<Track>,
    #[new(default)]
    #[serde(with = "serde_ord_map")]
    pub measure_lengths: OrdMap<BeatPosition, MeasureLength>,
    #[new(default)]
    #[serde(with = "serde_ord_map")]
    pub bpms: OrdMap<BeatPosition, Bpm>,
    #[new(default)]
    pub offset: f64,
    #[new(default)]
    pub lyrics: String,
    #[data(eq)]
    pub font_file: PathBuf,
}

impl Score {
    pub fn beat_to_time(&self, pos: &BeatPosition) -> f64 {
        beat_to_time(self.offset, &self.bpms, pos)
    }
    pub fn time_to_beat(&self, time: f64) -> f64 {
        time_to_beat(self.offset, &self.bpms, time)
    }
}
pub fn beat_to_time(offset: f64, bpms: &OrdMap<BeatPosition, Bpm>, pos: &BeatPosition) -> f64 {
    let mut time = offset;
    match bpms.iter().next() {
        None => return time + pos.0.to_f64().unwrap() / 2.0, // Assume BPM=120
        Some((first_beat, bpm)) => {
            time += first_beat.min(pos).0.to_f64().unwrap() * bpm.beat_length();
            if pos <= first_beat {
                return time;
            }
        }
    }
    for ((start_beat, bpm), (end_beat, _)) in bpms.iter().tuple_windows() {
        time += (end_beat.min(pos) - start_beat).0.to_f64().unwrap() * bpm.beat_length();
        if pos <= end_beat {
            return time;
        }
    }
    let (last_beat, bpm) = bpms.iter().next_back().expect("Always exists");
    time += (pos - last_beat).0.to_f64().unwrap() * bpm.beat_length();
    time
}

pub fn time_to_beat(offset: f64, bpms: &OrdMap<BeatPosition, Bpm>, time: f64) -> f64 {
    let mut cur_time = offset;
    match bpms.iter().next() {
        None => return (time - cur_time) * 2.0,
        Some((first_beat, bpm)) => {
            let first_beat = first_beat.0.to_f64().unwrap();
            let end_time = cur_time + first_beat * bpm.beat_length();
            if time <= end_time {
                return first_beat + (time - cur_time) / bpm.beat_length();
            }
            cur_time = end_time;
        }
    }
    for ((start_beat, bpm), (end_beat, _)) in bpms
        .iter()
        .map(|(beat, bpm)| (beat.0.to_f64().unwrap(), bpm))
        .tuple_windows()
    {
        let end_time = cur_time + (end_beat - start_beat) * bpm.beat_length();
        if time <= end_time {
            return start_beat + (time - cur_time) / bpm.beat_length();
        }
        cur_time = end_time;
    }
    let (last_beat, bpm) = bpms.iter().next_back().expect("Always exists");
    let last_beat = last_beat.0.to_f64().unwrap();
    last_beat + (time - cur_time) / bpm.beat_length()
}

#[derive(Clone, Debug, Serialize, Deserialize, Data)]
pub struct Track {
    pub start_beat: BeatPosition,
    pub elements: Vector<ScoreElement>,
    pub lyrics: Option<Lyrics>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Data)]
pub struct Lyrics {
    pub text: String,
    #[serde(with = "serde_ord_map")]
    pub mappings: OrdMap<(usize, usize), usize>,
}

// #[derive(Clone, Debug, Data, PartialEq, Eq, PartialOrd, Ord)]
// pub struct LyricsMapping {
//     pub start: usize,
//     pub end: usize,
//     pub divions: usize,
// }

impl Track {
    pub fn start_beat(&self) -> &BeatPosition {
        &self.start_beat
    }

    pub fn end_beat(&self) -> BeatPosition {
        self.elements
            .iter()
            .map(|x| &x.length)
            .fold(self.start_beat.to_owned(), |x, y| &x + y)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Data)]
pub struct ScoreElement {
    pub kind: ScoreElementKind,
    pub length: BeatLength,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Data)]
pub enum ScoreElementKind {
    Start,
    Stop,
    Skip,
}

impl Track {
    pub fn iterate_notes(
        &self,
    ) -> impl Iterator<Item = (BeatPosition, BeatPosition, &ScoreElement)> {
        use ScoreElementKind::*;
        let mut beat = self.start_beat.to_owned();
        let mut elements = self
            .elements
            .iter()
            .map(move |e| {
                let new_beat = &beat + &e.length;
                let old_beat = std::mem::replace(&mut beat, new_beat);
                (old_beat, e)
            })
            .peekable();

        iter::from_fn(move || {
            let (beat, note) = elements.find(|(_, e)| matches!(e.kind, Start))?;
            let end_beat = match elements
                .peeking_take_while(|(_, e)| matches!(e.kind, Skip))
                .last()
            {
                Some((end_beat, end_note)) => &end_beat + &end_note.length,
                None => &beat + &note.length,
            };
            Some((beat, end_beat, note))
        })
    }
}

pub fn iterate_measures<'a, BP, ML>(
    measures: impl Iterator<Item = (BP, ML)> + 'a,
) -> impl Iterator<Item = (BeatPosition, BeatPosition)> + 'a
where
    BP: Borrow<BeatPosition>,
    ML: Borrow<MeasureLength>,
{
    let mut measure_lengths = measures.peekable();
    let mut measure_length = BeatLength::four();
    let mut measure_start_beat = BeatPosition::zero();

    iter::from_fn(move || {
        let mut measure_end_beat = &measure_start_beat + &measure_length;
        if let Some((next_measure_beat, next_measure_length)) = measure_lengths.peek() {
            let next_measure_beat = next_measure_beat.borrow();
            let next_measure_length = next_measure_length.borrow();
            match next_measure_beat.cmp(&measure_start_beat) {
                Equal => {
                    measure_length = (*next_measure_length).to_owned().into();
                    measure_end_beat = &measure_start_beat + &measure_length;
                    measure_lengths.next();
                }
                Less => {
                    measure_length = (*next_measure_length).to_owned().into();
                    measure_end_beat = (*next_measure_beat).to_owned();
                    measure_lengths.next();
                }
                Greater => {}
            }
        }
        let ret = Some((measure_start_beat.clone(), measure_end_beat.clone()));
        measure_start_beat = measure_end_beat;
        ret
    })
}

pub fn iterate_beat_times(
    offset: f64,
    measures: OrdMap<BeatPosition, MeasureLength>,
    bpms: OrdMap<BeatPosition, Bpm>,
    start_beat: BeatPosition,
) -> impl Iterator<Item = (bool, f64)> {
    let mut beat = start_beat;
    let mut time = beat_to_time(offset, &bpms, &beat);
    let mut bpms = bpms.into_iter().peekable();
    let mut bpm = match bpms.peeking_take_while(|(b, _)| b <= &beat).last() {
        Some((_, bpm)) => bpm,
        None => bpms.peek().map_or_else(Default::default, |b| b.1),
    };
    let mut measures = iterate_measures(measures.into_iter()).peekable();
    measures.peeking_take_while(|(_, end)| end <= &beat).count();
    let mut first_in_measure = measures.peek().expect("iterate_measure is infinite").0 == beat;
    // dbg!(&beat, &time);
    // dbg!(bpms.peek());

    iter::from_fn(move || {
        let ret = Some((first_in_measure, time));
        // println!("{:?}", ret);
        let next_beat_in_measure = &beat + &BeatLength::one();
        let next_measure_beat = &measures.peek().expect("iterate_measures is infinite").1;
        let (next_first_in_measure, next_beat) = match next_measure_beat.cmp(&next_beat_in_measure)
        {
            Less | Equal => {
                measures.next();
                (
                    true,
                    &measures.peek().expect("iterate_measures is infinite").0,
                )
            }
            Greater => (false, &next_beat_in_measure),
        };
        while let Some((next_bpm_beat, next_bpm)) = bpms.peek().filter(|b| &b.0 < next_beat) {
            // println!("  {:?} {:?} => {:?} {:?}", beat, bpm, next_bpm_beat, next_bpm);
            time += bpm * (next_bpm_beat - &beat);
            beat = next_bpm_beat.clone();
            bpm = *next_bpm;
            bpms.next();
        }
        time += bpm * (next_beat - &beat);
        beat = next_beat.clone();
        first_in_measure = next_first_in_measure;
        // println!("   => {:?} {:?}", time, beat);
        ret
    })
}

#[cfg(test)]
mod test {
    use std::iter;

    use super::beat_to_time;
    use super::iterate_beat_times;
    use super::iterate_measures;
    use super::BeatPosition;
    use super::Bpm;
    use super::MeasureLength;
    use druid::im::ordmap;
    use itertools::iterate;
    use itertools::Itertools;
    use num_rational::BigRational;

    macro_rules! bp {
        ($a: expr) => {
            BeatPosition::from(BigRational::from_integer($a.into()))
        };
    }

    #[test]
    fn test_iterate_measures_01() {
        let got = iterate_measures(iter::empty::<(BeatPosition, MeasureLength)>())
            .take(10)
            .collect_vec();
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
            BeatPosition::from(BigRational::from_integer(16.into())) => MeasureLength::new(3, 4)
        ];
        let got = iterate_measures(measures.into_iter())
            .take(10)
            .collect_vec();
        let expected = vec![0, 4, 8, 12, 16, 19, 22, 25, 28, 31, 34]
            .into_iter()
            .map(|x| BeatPosition::from(BigRational::from_integer(x.into())))
            .tuple_windows::<(_, _)>()
            .collect_vec();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_iterate_measures_03() {
        let measures = ordmap![
            BeatPosition::zero() => MeasureLength::new(3, 4)
        ];
        let got = iterate_measures(measures.into_iter())
            .take(10)
            .collect_vec();
        let expected = iterate(0, |x| x + 3)
            .map(|x| BeatPosition::from(BigRational::from_integer(x.into())))
            .tuple_windows::<(_, _)>()
            .take(10)
            .collect_vec();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_iterate_beat_times_01() {
        let measures = ordmap![];
        let bpms = ordmap![];
        let got = iterate_beat_times(0.0, measures, bpms, BeatPosition::zero())
            .take(10)
            .collect_vec();
        let expected = iterate(0.0, |x| x + 0.5)
            .enumerate()
            .map(|(i, x)| (i % 4 == 0, x))
            .take(10)
            .collect_vec();
        assert_eq!(got, expected);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_beat_to_time_01() {
        let offset = 2.5;
        let bpms = ordmap![
            bp!(8) => Bpm(240.0),
            bp!(16) => Bpm(120.0),
            BeatPosition::from(BigRational::new(41.into(), 2.into())) => Bpm(240.0) // 20.5
        ];
        assert_eq!(beat_to_time(offset, &bpms, &bp!(0)), 2.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(1)), 2.75);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(2)), 3.0);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(3)), 3.25);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(4)), 3.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(5)), 3.75);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(6)), 4.0);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(7)), 4.25);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(8)), 4.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(9)), 4.75);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(10)), 5.00);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(11)), 5.25);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(12)), 5.50);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(13)), 5.75);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(14)), 6.00);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(15)), 6.25);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(16)), 6.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(17)), 7.0);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(18)), 7.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(19)), 8.0);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(20)), 8.5);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(21)), 8.875);
        assert_eq!(beat_to_time(offset, &bpms, &bp!(22)), 9.125);
    }

    #[test]
    fn test_iterate_beat_times_02() {
        let measures = ordmap![
            BeatPosition::from(BigRational::from_integer(16.into())) => MeasureLength::new(3, 4)
        ];
        let bpms = ordmap![
            BeatPosition::from(BigRational::from_integer(8.into())) => Bpm(240.0),
            BeatPosition::from(BigRational::from_integer(22.into())) => Bpm(120.0),
            BeatPosition::from(BigRational::new(51.into(), 2.into())) => Bpm(240.0) // 25.5
        ];
        let got = iterate_beat_times(2.5, measures, bpms, BeatPosition::zero())
            .take(16 + 12)
            .collect_vec();
        assert_eq!(
            got,
            vec![
                (true, 2.5),
                (false, 2.75),
                (false, 3.0),
                (false, 3.25),
                (true, 3.5),
                (false, 3.75),
                (false, 4.0),
                (false, 4.25),
                (true, 4.5),
                (false, 4.75),
                (false, 5.0),
                (false, 5.25),
                (true, 5.5),
                (false, 5.75),
                (false, 6.0),
                (false, 6.25),
                (true, 6.5),
                (false, 6.75),
                (false, 7.0),
                (true, 7.25),
                (false, 7.5),
                (false, 7.75),
                (true, 8.0),
                (false, 8.5),
                (false, 9.0),
                (true, 9.5),
                (false, 9.875),
                (false, 10.125),
            ]
        );
    }

    #[test]
    fn test_iterate_beat_times_03() {
        let measures = ordmap![
            BeatPosition::from(BigRational::from_integer(16.into())) => MeasureLength::new(3, 4)
        ];
        let bpms = ordmap![
            BeatPosition::from(BigRational::from_integer(8.into())) => Bpm(240.0),
            BeatPosition::from(BigRational::from_integer(22.into())) => Bpm(120.0),
            BeatPosition::from(BigRational::new(51.into(), 2.into())) => Bpm(240.0) // 25.5
        ];
        let got = iterate_beat_times(
            2.5,
            measures,
            bpms,
            BeatPosition::from(BigRational::from_integer(11.into())),
        )
        .take(16 + 12 - 11)
        .collect_vec();
        assert_eq!(
            got,
            vec![
                (false, 5.25),
                (true, 5.5),
                (false, 5.75),
                (false, 6.0),
                (false, 6.25),
                (true, 6.5),
                (false, 6.75),
                (false, 7.0),
                (true, 7.25),
                (false, 7.5),
                (false, 7.75),
                (true, 8.0),
                (false, 8.5),
                (false, 9.0),
                (true, 9.5),
                (false, 9.875),
                (false, 10.125),
            ]
        );
    }

    #[test]
    fn test_iterate_beat_times_04() {
        let measures = ordmap![];
        let bpms = ordmap![
            bp!(0) => Bpm(480.0),
            bp!(8) => Bpm(240.0),
            bp!(16) => Bpm(120.0)
        ];
        let got = iterate_beat_times(0.5, measures, bpms, bp!(12))
            .take(8)
            .collect_vec();
        let expected = vec![
            (true, 2.5),
            (false, 2.75),
            (false, 3.0),
            (false, 3.25),
            (true, 3.5),
            (false, 4.0),
            (false, 4.5),
            (false, 5.0),
        ];
        assert_eq!(got, expected);
    }
}
