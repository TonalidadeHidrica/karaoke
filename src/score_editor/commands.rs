use crate::schema::BeatPosition;
use crate::schema::Bpm;
use crate::schema::MeasureLength;
use druid::Selector;
use druid::SingleUse;

#[derive(Debug)]
pub struct SetMeasureLengthCommand {
    pub position: BeatPosition,
    pub measure_length: Option<MeasureLength>,
}

selector! { pub EDIT_MEAUSRE_LENGTH_SELECTOR: SingleUse<SetMeasureLengthCommand> }

pub struct SetBpmCommand {
    pub position: BeatPosition,
    pub bpm: Option<Bpm>,
}

selector! { pub EDIT_BPM_SELECTOR: SingleUse<SetBpmCommand> }
