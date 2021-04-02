use druid::im::Vector;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::TextBox;
use druid::Data;
use druid::Lens;
use druid::Widget;
use druid::WidgetExt;

use crate::linest::Linest;
use crate::linest::LinestResult;

#[derive(Clone, Default, Debug, Data, Lens)]
pub struct BpmDetectorData {
    detected_bpm: String,
    detected_offset: String,
    cues: Vector<f64>,
    linest: Linest,
    linest_result: Option<LinestResult>,
}

impl BpmDetectorData {
    pub fn push(&mut self, time: f64) {
        self.linest.push(self.cues.len() as f64, time);
        self.linest_result = self.linest.estimate();
        self.cues.push_back(time);
        if let Some(res) = self.linest_result {
            self.detected_bpm = format!("{}", 60.0 / res.a);
            self.detected_offset = format!("{}", res.b);
        } else {
            self.detected_bpm.clear();
            self.detected_offset.clear();
        }
    }
}

pub fn build_bpm_detector_widget() -> impl Widget<BpmDetectorData> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_child(Label::new("Detected BPM:"))
                .with_child(TextBox::new().lens(BpmDetectorData::detected_bpm)),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Detected offset:"))
                .with_child(TextBox::new().lens(BpmDetectorData::detected_offset)),
        )
        .with_child(Label::dynamic(|data: &BpmDetectorData, _| {
            match data.linest_result {
                None => "".to_owned(),
                Some(res) => format!("R^2 = {}", res.r2),
            }
        }))
}
