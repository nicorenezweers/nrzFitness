use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: String,
    /// 'm' | 'f'
    pub sex: String,
    pub birth_date: Option<NaiveDate>,
    pub height_cm: Option<f64>,
    pub activity_level: String,
    pub cycle_tracking: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileInput {
    pub name: String,
    pub color: String,
    pub sex: String,
    pub birth_date: Option<NaiveDate>,
    pub height_cm: Option<f64>,
    pub activity_level: String,
    pub cycle_tracking: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Goals {
    pub calories: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub fiber_g: f64,
    pub sugar_g: f64,
    pub water_ml: f64,
    pub steps: i64,
    pub exercise_min: i64,
    pub sleep_hours: f64,
    pub target_weight: Option<f64>,
    pub weekly_change: f64,
    pub fasting_hours: f64,
}

impl Default for Goals {
    fn default() -> Self {
        Self {
            calories: 2000.0,
            protein_g: 120.0,
            carbs_g: 220.0,
            fat_g: 70.0,
            fiber_g: 30.0,
            sugar_g: 50.0,
            water_ml: 2500.0,
            steps: 8000,
            exercise_min: 30,
            sleep_hours: 8.0,
            target_weight: None,
            weekly_change: 0.0,
            fasting_hours: 16.0,
        }
    }
}

impl Profile {
    pub fn age(&self) -> Option<i32> {
        self.birth_date.map(|b| crate::util::age_on(b, crate::util::today()))
    }

    /// Gesamtumsatz (TDEE) aus Profil und Gewicht, falls Grössen bekannt.
    pub fn tdee(&self, weight_kg: f64) -> Option<f64> {
        let h = self.height_cm.filter(|h| *h > 0.0)?;
        let age = self.age().unwrap_or(30);
        Some(crate::util::bmr(&self.sex, weight_kg, h, age) * crate::util::activity_factor(&self.activity_level))
    }
}

/// Zielvorschlag: kcal = TDEE + Wochenänderung (7700 kcal/kg), Protein 1.8 g/kg, Fett 27 %, Rest Kohlenhydrate.
pub fn suggested_goals(p: &Profile, weight_kg: f64, weekly_change: f64, base: &Goals) -> Goals {
    let tdee = p.tdee(weight_kg).unwrap_or(2000.0);
    let floor = if p.sex == "f" { 1200.0 } else { 1500.0 };
    let kcal = (tdee + weekly_change * 7700.0 / 7.0).max(floor);
    let kcal = (kcal / 10.0).round() * 10.0;
    let protein = (weight_kg * if weekly_change < 0.0 { 2.0 } else { 1.8 }).round();
    let fat = (kcal * 0.27 / 9.0).round();
    let carbs = ((kcal - protein * 4.0 - fat * 9.0) / 4.0).max(50.0).round();
    let water = ((weight_kg * 35.0) / 250.0).round() * 250.0;
    Goals {
        calories: kcal,
        protein_g: protein,
        carbs_g: carbs,
        fat_g: fat,
        fiber_g: if p.sex == "f" { 25.0 } else { 30.0 },
        water_ml: water.clamp(1500.0, 4500.0),
        weekly_change,
        ..base.clone()
    }
}
