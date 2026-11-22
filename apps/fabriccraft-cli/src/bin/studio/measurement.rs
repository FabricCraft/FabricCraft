use eframe::egui;

const UNITS: [(&str, f64); 6] = [
    ("mm", 1.),
    ("cm", 10.),
    ("m", 1000.),
    ("in", 25.4),
    ("ft", 304.8),
    ("yd", 914.4),
];

#[derive(Default)]
pub struct Measurement {
    pub text: String,
    pub unit: usize,
}

impl Measurement {
    pub fn millimetres(&self) -> Option<f64> {
        let mut parts = self.text.split_whitespace();
        let first = parts.next()?;
        let value = match (parts.next(), parts.next()) {
            (None, None) => first.parse().ok().or_else(|| fraction(first))?,
            (Some(part), None) => {
                let whole = first.parse::<u64>().ok()? as f64;
                whole + fraction(part)?
            }
            _ => return None,
        };
        let mm = value * UNITS[self.unit].1;
        (mm.is_finite() && mm >= 0.).then_some(mm)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.text)
                        .desired_width(66.)
                        .hint_text("0"),
                )
                .changed();
            egui::ComboBox::from_id_salt(label)
                .selected_text(UNITS[self.unit].0)
                .width(40.)
                .show_ui(ui, |ui| {
                    for (index, (name, _)) in UNITS.iter().enumerate() {
                        changed |= ui.selectable_value(&mut self.unit, index, *name).changed();
                    }
                });
            ui.label(label);
        });
        changed
    }
}

fn fraction(value: &str) -> Option<f64> {
    let (numerator, denominator) = value.split_once('/')?;
    let numerator = numerator.parse::<u64>().ok()? as f64;
    let denominator = denominator.parse::<u64>().ok()? as f64;
    (denominator > 0.).then_some(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_metric_imperial_and_fractional_measurements() {
        for (text, unit, expected) in [
            ("12.7", 0, 12.7),
            ("1.27", 1, 12.7),
            ("0.0127", 2, 12.7),
            ("0.5", 3, 12.7),
            ("1/2", 3, 12.7),
            ("1 1/2", 3, 38.1),
            ("1", 4, 304.8),
            ("1", 5, 914.4),
        ] {
            let input = Measurement {
                text: text.into(),
                unit,
            };
            assert!((input.millimetres().unwrap() - expected).abs() < 1e-9);
        }
        for text in ["-0.5", "-1/2", "1/0", "nan", "inf", "1 1/2 3"] {
            let input = Measurement {
                text: text.into(),
                unit: 3,
            };
            assert!(input.millimetres().is_none());
        }
    }
}
