use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Mahlzeiten im Tagebuch (Schlüssel, Anzeigename).
pub const MEALS: [(&str, &str); 4] =
    [("breakfast", "Frühstück"), ("lunch", "Mittagessen"), ("dinner", "Abendessen"), ("snack", "Snacks")];

pub fn meal_label(key: &str) -> &'static str {
    MEALS.iter().find(|(k, _)| *k == key).map(|(_, l)| *l).unwrap_or("Snacks")
}

/// Vorschlag für die Mahlzeit nach Uhrzeit.
pub fn meal_for_hour(h: u32) -> &'static str {
    match h {
        4..=10 => "breakfast",
        11..=14 => "lunch",
        17..=21 => "dinner",
        _ => "snack",
    }
}

/// Lebensmittel; Nährwerte pro 100 g bzw. 100 ml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Food {
    pub id: String,
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub unit: String,
    pub serving_size: f64,
    pub serving_name: Option<String>,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sugar: f64,
    pub sat_fat: f64,
    pub sodium_mg: f64,
    pub source: String,
    pub image_url: Option<String>,
    #[cfg_attr(feature = "server", sqlx(default))]
    pub favorite: bool,
    /// Für alle Profile sichtbar (profile_id NULL)?
    #[cfg_attr(feature = "server", sqlx(default))]
    pub shared: bool,
}

impl Food {
    /// Nährwerte für eine Menge (g/ml).
    pub fn scaled(&self, amount: f64) -> Nutrients {
        let f = amount / 100.0;
        Nutrients {
            kcal: self.kcal * f,
            protein: self.protein * f,
            carbs: self.carbs * f,
            fat: self.fat * f,
            fiber: self.fiber * f,
            sugar: self.sugar * f,
            sodium_mg: self.sodium_mg * f,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Nutrients {
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sugar: f64,
    pub sodium_mg: f64,
}

impl std::ops::AddAssign for Nutrients {
    fn add_assign(&mut self, o: Self) {
        self.kcal += o.kcal;
        self.protein += o.protein;
        self.carbs += o.carbs;
        self.fat += o.fat;
        self.fiber += o.fiber;
        self.sugar += o.sugar;
        self.sodium_mg += o.sodium_mg;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FoodInput {
    pub name: String,
    pub brand: Option<String>,
    pub barcode: Option<String>,
    pub unit: String,
    pub serving_size: f64,
    pub serving_name: Option<String>,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sugar: f64,
    pub sat_fat: f64,
    pub sodium_mg: f64,
    pub source: String,
    pub image_url: Option<String>,
    pub shared: bool,
}

impl From<&Food> for FoodInput {
    fn from(f: &Food) -> Self {
        Self {
            name: f.name.clone(),
            brand: f.brand.clone(),
            barcode: f.barcode.clone(),
            unit: f.unit.clone(),
            serving_size: f.serving_size,
            serving_name: f.serving_name.clone(),
            kcal: f.kcal,
            protein: f.protein,
            carbs: f.carbs,
            fat: f.fat,
            fiber: f.fiber,
            sugar: f.sugar,
            sat_fat: f.sat_fat,
            sodium_mg: f.sodium_mg,
            source: f.source.clone(),
            image_url: f.image_url.clone(),
            shared: f.shared,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct FoodEntry {
    pub id: String,
    pub date: NaiveDate,
    pub meal: String,
    pub food_id: Option<String>,
    pub name: String,
    pub brand: Option<String>,
    pub unit: String,
    pub amount: f64,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sugar: f64,
    pub sodium_mg: f64,
}

impl FoodEntry {
    pub fn nutrients(&self) -> Nutrients {
        Nutrients {
            kcal: self.kcal,
            protein: self.protein,
            carbs: self.carbs,
            fat: self.fat,
            fiber: self.fiber,
            sugar: self.sugar,
            sodium_mg: self.sodium_mg,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct WaterEntry {
    pub id: String,
    pub ml: f64,
    pub created_at: chrono::NaiveDateTime,
}

/// Mahlzeiten-Vorlage (Rezept).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealTemplate {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    pub items: Vec<MealItem>,
}

impl MealTemplate {
    pub fn totals(&self) -> Nutrients {
        let mut n = Nutrients::default();
        for i in &self.items {
            n += i.food.scaled(i.amount);
        }
        n
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealItem {
    pub id: String,
    pub food: Food,
    pub amount: f64,
}

/// Alles, was die Heute-Seite für einen Tag braucht.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DayData {
    pub entries: Vec<FoodEntry>,
    pub water_ml: f64,
    pub water: Vec<WaterEntry>,
    /// Verbrannte kcal und Minuten aus Training.
    pub burned_kcal: f64,
    pub exercise_min: f64,
    pub exercise_count: i64,
    pub steps: Option<i64>,
    pub weight: Option<f64>,
    pub mood: Option<i64>,
    pub sleep_hours: Option<f64>,
    /// Laufendes Fasten (Start, Ziel in h).
    pub fasting_since: Option<chrono::NaiveDateTime>,
    pub fasting_target: Option<f64>,
}

impl DayData {
    pub fn totals(&self) -> Nutrients {
        let mut n = Nutrients::default();
        for e in &self.entries {
            n += e.nutrients();
        }
        n
    }
}

/// Ergebnis der Online-Suche (Open Food Facts), noch nicht gespeichert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FoodSearch {
    pub local: Vec<Food>,
    pub online: Vec<FoodInput>,
}
