//! The damage meter of the session, as the DPS window shows it: the damage
//! the character's side dealt to each mobile, and each second.

use serde_json::Value;

/// One mobile the meter counted.
#[derive(Clone, Debug, PartialEq)]
pub struct Dealt {
    pub name: String,
    pub damage: u64,
    pub per_second: f64,
}

/// A report of the meter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageReport {
    pub running: bool,
    pub seconds: f64,
    pub mobiles: Vec<Dealt>,
}

impl DamageReport {
    /// Reads the answer of the `damage_meter` tool.
    pub fn read(answer: &Value) -> Self {
        let mobiles = answer
            .get("mobiles")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|row| Dealt {
                        name: row
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        damage: row
                            .get("damage")
                            .and_then(Value::as_u64)
                            .unwrap_or_default(),
                        per_second: row
                            .get("per_second")
                            .and_then(Value::as_f64)
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            running: answer
                .get("running")
                .and_then(Value::as_bool)
                .unwrap_or_default(),
            seconds: answer
                .get("seconds")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            mobiles,
        }
    }

    pub fn total(&self) -> u64 {
        self.mobiles.iter().map(|dealt| dealt.damage).sum()
    }

    /// The damage of all mobiles each second, over the time the meter ran.
    pub fn per_second(&self) -> f64 {
        if self.seconds > 0.0 {
            self.total() as f64 / self.seconds
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_report_reads_and_sums() {
        let report = DamageReport::read(&json!({
            "running": true,
            "seconds": 10.0,
            "mobiles": [
                { "serial": 1, "name": "an orc", "damage": 80, "per_second": 8.0 },
                { "serial": 2, "name": "a troll", "damage": 20, "per_second": 2.0 },
            ],
        }));
        assert!(report.running);
        assert_eq!(report.total(), 100);
        assert!((report.per_second() - 10.0).abs() < f64::EPSILON);
        assert_eq!(report.mobiles[0].name, "an orc");
        assert_eq!(DamageReport::read(&json!({})), DamageReport::default());
        assert_eq!(DamageReport::default().per_second(), 0.0);
    }
}
