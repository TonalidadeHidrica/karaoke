use std::iter;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;

use derive_more::From;
use druid::im::OrdMap;
use druid::im::Vector;
use druid::Data;
use druid::Lens;
use itertools::Itertools;
use num::rational::BigRational;
use num::BigInt;
use num::One;
use num::ToPrimitive;
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

#[derive(Clone, Debug, derive_more::From, derive_more::FromStr, derive_more::Display)]
pub struct BigIntData(BigInt);

impl Data for BigIntData {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug, derive_more::Display, Data, Lens)]
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

#[derive(Clone, Copy, Debug, derive_more::FromStr, derive_more::Display, Data)]
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

#[derive(Clone, Default, Debug, Data, Lens)]
pub struct Score {
    pub tracks: Vector<Track>,
    pub measure_lengths: OrdMap<BeatPosition, MeasureLength>,
    pub bpms: OrdMap<BeatPosition, Bpm>,
    pub offset: f64,
}

impl Score {
    pub fn beat_to_time(&self, pos: &BeatPosition) -> f64 {
        let mut time = self.offset;
        match self.bpms.iter().next() {
            None => return time + pos.0.to_f64().unwrap() / 2.0, // Assume BPM=120
            Some((first_beat, bpm)) => {
                time += first_beat.min(pos).0.to_f64().unwrap() * bpm.beat_length();
                if pos <= first_beat {
                    return time;
                }
            }
        }
        for ((start_beat, bpm), (end_beat, _)) in self.bpms.iter().tuple_windows() {
            time += (end_beat.min(pos) - start_beat).0.to_f64().unwrap() * bpm.beat_length();
            if pos <= end_beat {
                return time;
            }
        }
        let (last_beat, bpm) = self.bpms.iter().next_back().expect("Always exists");
        time += (pos - last_beat).0.to_f64().unwrap() * bpm.beat_length();
        time
    }

    pub fn time_to_beat(&self, time: f64) -> f64 {
        let mut cur_time = self.offset;
        match self.bpms.iter().next() {
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
        for ((start_beat, bpm), (end_beat, _)) in self
            .bpms
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
        let (last_beat, bpm) = self.bpms.iter().next_back().expect("Always exists");
        let last_beat = last_beat.0.to_f64().unwrap();
        last_beat + (time - cur_time) / bpm.beat_length()
    }
}

#[derive(Clone, Debug, Data)]
pub struct Track {
    pub start_beat: BeatPosition,
    pub elements: Vector<ScoreElement>,
}

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

#[derive(Clone, PartialEq, Debug, Data)]
pub struct ScoreElement {
    pub kind: ScoreElementKind,
    pub length: BeatLength,
}

#[derive(Clone, Copy, PartialEq, Debug, Data)]
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

pub fn iterate_measures(
    measures: &OrdMap<BeatPosition, MeasureLength>,
) -> impl Iterator<Item = (BeatPosition, BeatPosition)> + '_ {
    let mut measure_lengths = measures.iter().peekable();
    let mut measure_length = BeatLength::four();
    let mut measure_start_beat = BeatPosition::zero();

    iter::from_fn(move || {
        let mut measure_end_beat = &measure_start_beat + &measure_length;
        if let Some((next_measure_beat, next_measure_length)) = measure_lengths.peek() {
            if next_measure_beat == &&measure_start_beat {
                measure_length = (*next_measure_length).to_owned().into();
                measure_end_beat = &measure_start_beat + &measure_length;
                measure_lengths.next();
            } else if next_measure_beat < &&measure_end_beat {
                measure_length = (*next_measure_length).to_owned().into();
                measure_end_beat = (*next_measure_beat).to_owned();
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
    use super::iterate_measures;
    use super::BeatPosition;
    use super::MeasureLength;
    use druid::im::ordmap;
    use itertools::iterate;
    use itertools::Itertools;
    use num::BigRational;

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
            BeatPosition::from(BigRational::from_integer(16.into())) => MeasureLength::new(3, 4)
        ];
        let got = iterate_measures(&measures).take(10).collect_vec();
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
        let got = iterate_measures(&measures).take(10).collect_vec();
        let expected = iterate(0, |x| x + 3)
            .map(|x| BeatPosition::from(BigRational::from_integer(x.into())))
            .tuple_windows::<(_, _)>()
            .take(10)
            .collect_vec();
        assert_eq!(got, expected);
    }
}
