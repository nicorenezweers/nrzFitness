//! Basis-Lebensmittel (generisch, Richtwerte pro 100 g/ml nach gängigen Nährwerttabellen).
//! Werden einmalig für alle Profile angelegt (feste IDs, INSERT OR IGNORE).

use sqlx::SqlitePool;

use super::now_ts;

// (id, Name, Einheit, Portion, Portionsname, kcal, Protein, Kohlenhydrate, Fett, Ballaststoffe, Zucker)
#[rustfmt::skip]
const FOODS: &[(&str, &str, &str, f64, &str, f64, f64, f64, f64, f64, f64)] = &[
    // Getreide & Beilagen
    ("haferflocken", "Haferflocken", "g", 50.0, "1 Portion", 372.0, 13.5, 58.7, 7.0, 10.0, 0.7),
    ("muesli", "Müesli (ungesüsst)", "g", 60.0, "1 Schale", 365.0, 10.0, 60.0, 7.5, 8.5, 12.0),
    ("cornflakes", "Cornflakes", "g", 30.0, "1 Portion", 378.0, 7.0, 84.0, 0.9, 3.0, 8.0),
    ("reis-roh", "Reis, roh", "g", 75.0, "1 Portion", 350.0, 7.0, 78.0, 0.6, 1.4, 0.2),
    ("reis-gekocht", "Reis, gekocht", "g", 200.0, "1 Portion", 130.0, 2.7, 28.0, 0.3, 0.4, 0.1),
    ("basmati-roh", "Basmatireis, roh", "g", 75.0, "1 Portion", 352.0, 8.0, 77.0, 0.9, 1.0, 0.1),
    ("nudeln-roh", "Teigwaren, roh", "g", 100.0, "1 Portion", 357.0, 12.5, 71.0, 1.5, 3.0, 3.0),
    ("nudeln-gekocht", "Teigwaren, gekocht", "g", 220.0, "1 Portion", 150.0, 5.5, 30.0, 0.8, 1.8, 0.6),
    ("vollkornnudeln-roh", "Vollkornteigwaren, roh", "g", 100.0, "1 Portion", 348.0, 13.0, 64.0, 2.5, 8.0, 3.0),
    ("kartoffeln", "Kartoffeln, gekocht", "g", 200.0, "1 Portion", 77.0, 2.0, 17.0, 0.1, 1.8, 0.8),
    ("suesskartoffel", "Süsskartoffel, gekocht", "g", 200.0, "1 Portion", 86.0, 1.6, 20.0, 0.1, 3.0, 4.2),
    ("pommes", "Pommes frites", "g", 150.0, "1 Portion", 312.0, 3.4, 41.0, 15.0, 3.8, 0.3),
    ("quinoa-roh", "Quinoa, roh", "g", 70.0, "1 Portion", 368.0, 14.0, 64.0, 6.0, 7.0, 0.0),
    ("couscous-roh", "Couscous, roh", "g", 70.0, "1 Portion", 376.0, 12.8, 77.0, 0.6, 5.0, 0.0),
    ("bulgur-roh", "Bulgur, roh", "g", 70.0, "1 Portion", 342.0, 12.0, 76.0, 1.3, 12.5, 0.4),
    ("polenta-roh", "Polenta (Maisgriess), roh", "g", 60.0, "1 Portion", 350.0, 8.0, 74.0, 1.5, 4.0, 0.6),
    // Brot & Backwaren
    ("vollkornbrot", "Vollkornbrot", "g", 50.0, "1 Scheibe", 220.0, 8.0, 38.0, 1.6, 7.0, 3.0),
    ("ruchbrot", "Ruchbrot / Mischbrot", "g", 50.0, "1 Scheibe", 240.0, 8.5, 46.0, 1.5, 4.5, 2.0),
    ("weissbrot", "Weissbrot / Toast", "g", 30.0, "1 Scheibe", 265.0, 8.5, 49.0, 3.2, 2.7, 4.0),
    ("zopf", "Zopf", "g", 50.0, "1 Scheibe", 320.0, 9.0, 50.0, 9.0, 2.0, 6.0),
    ("gipfeli", "Gipfeli (Croissant)", "g", 50.0, "1 Stück", 406.0, 8.2, 45.0, 21.0, 2.6, 8.0),
    ("broetchen", "Brötchen / Semmel", "g", 60.0, "1 Stück", 270.0, 9.0, 52.0, 2.0, 3.0, 3.0),
    ("knaeckebrot", "Knäckebrot", "g", 10.0, "1 Scheibe", 350.0, 10.0, 66.0, 2.0, 16.0, 2.0),
    ("tortilla", "Tortilla (Weizen)", "g", 60.0, "1 Stück", 310.0, 8.5, 50.0, 8.0, 3.0, 2.5),
    ("pizza-margherita", "Pizza Margherita", "g", 350.0, "1 Pizza", 250.0, 10.5, 32.0, 8.5, 2.0, 3.5),
    // Milchprodukte & Eier
    ("milch-voll", "Vollmilch 3,5 %", "ml", 250.0, "1 Glas", 64.0, 3.3, 4.8, 3.5, 0.0, 4.8),
    ("milch-fettarm", "Milch 1,5 %", "ml", 250.0, "1 Glas", 47.0, 3.4, 4.9, 1.5, 0.0, 4.9),
    ("hafermilch", "Haferdrink", "ml", 250.0, "1 Glas", 46.0, 0.7, 7.0, 1.5, 0.8, 4.0),
    ("sojamilch", "Sojadrink (ungesüsst)", "ml", 250.0, "1 Glas", 33.0, 3.3, 0.3, 1.8, 0.6, 0.2),
    ("joghurt-nature", "Joghurt nature 3,5 %", "g", 180.0, "1 Becher", 66.0, 3.8, 4.7, 3.5, 0.0, 4.7),
    ("griechisches-joghurt", "Griechisches Joghurt 10 %", "g", 150.0, "1 Portion", 122.0, 4.5, 4.0, 10.0, 0.0, 4.0),
    ("skyr", "Skyr nature", "g", 150.0, "1 Portion", 63.0, 11.0, 4.0, 0.2, 0.0, 4.0),
    ("magerquark", "Magerquark", "g", 250.0, "1 Becher", 67.0, 12.0, 4.0, 0.3, 0.0, 4.0),
    ("huettenkaese", "Hüttenkäse", "g", 200.0, "1 Becher", 98.0, 12.5, 3.0, 4.3, 0.0, 2.7),
    ("mozzarella", "Mozzarella", "g", 125.0, "1 Kugel", 254.0, 18.0, 1.0, 19.5, 0.0, 1.0),
    ("gruyere", "Gruyère / Hartkäse", "g", 30.0, "1 Portion", 413.0, 27.0, 0.0, 33.0, 0.0, 0.0),
    ("emmentaler", "Emmentaler", "g", 30.0, "1 Portion", 380.0, 28.5, 0.0, 29.5, 0.0, 0.0),
    ("feta", "Feta", "g", 50.0, "1 Portion", 264.0, 14.0, 4.0, 21.0, 0.0, 4.0),
    ("frischkaese", "Frischkäse", "g", 30.0, "1 Portion", 250.0, 6.0, 3.5, 24.0, 0.0, 3.5),
    ("butter", "Butter", "g", 10.0, "1 Portion", 741.0, 0.7, 0.6, 82.0, 0.0, 0.6),
    ("rahm", "Vollrahm 35 %", "ml", 50.0, "1 Portion", 337.0, 2.1, 3.0, 35.0, 0.0, 3.0),
    ("ei", "Ei", "g", 60.0, "1 Stück (M)", 143.0, 12.6, 0.7, 9.5, 0.0, 0.4),
    ("eiweiss", "Eiklar", "g", 33.0, "1 Eiklar", 52.0, 10.9, 0.7, 0.2, 0.0, 0.7),
    // Fleisch & Fisch
    ("pouletbrust", "Pouletbrust, roh", "g", 150.0, "1 Stück", 110.0, 23.0, 0.0, 1.5, 0.0, 0.0),
    ("pouletschenkel", "Pouletschenkel, roh", "g", 180.0, "1 Stück", 170.0, 18.0, 0.0, 11.0, 0.0, 0.0),
    ("rindhack", "Rindshackfleisch, roh", "g", 125.0, "1 Portion", 250.0, 17.0, 0.0, 20.0, 0.0, 0.0),
    ("rindsteak", "Rindsteak, roh", "g", 200.0, "1 Steak", 140.0, 22.0, 0.0, 5.5, 0.0, 0.0),
    ("schweinsfilet", "Schweinsfilet, roh", "g", 150.0, "1 Portion", 120.0, 21.0, 0.0, 4.0, 0.0, 0.0),
    ("truthahn", "Truthahnbrust, roh", "g", 150.0, "1 Portion", 105.0, 24.0, 0.0, 1.0, 0.0, 0.0),
    ("schinken", "Schinken, gekocht", "g", 30.0, "2 Scheiben", 115.0, 19.0, 1.0, 3.5, 0.0, 1.0),
    ("salami", "Salami", "g", 20.0, "4 Scheiben", 400.0, 22.0, 1.0, 34.0, 0.0, 0.5),
    ("cervelat", "Cervelat", "g", 100.0, "1 Stück", 270.0, 12.0, 1.5, 24.0, 0.0, 0.5),
    ("lachs", "Lachs, roh", "g", 150.0, "1 Filet", 208.0, 20.0, 0.0, 13.0, 0.0, 0.0),
    ("thunfisch-dose", "Thunfisch in Wasser (abgetropft)", "g", 120.0, "1 Dose", 116.0, 26.0, 0.0, 1.0, 0.0, 0.0),
    ("kabeljau", "Kabeljau / Dorsch, roh", "g", 150.0, "1 Filet", 82.0, 18.0, 0.0, 0.7, 0.0, 0.0),
    ("crevetten", "Crevetten, gekocht", "g", 100.0, "1 Portion", 99.0, 24.0, 0.2, 0.3, 0.0, 0.0),
    // Pflanzliche Proteine
    ("tofu", "Tofu natur", "g", 100.0, "1 Portion", 144.0, 15.0, 2.0, 8.5, 1.0, 0.6),
    ("linsen-roh", "Linsen, roh", "g", 70.0, "1 Portion", 330.0, 25.0, 50.0, 1.5, 11.0, 2.0),
    ("linsen-gekocht", "Linsen, gekocht", "g", 200.0, "1 Portion", 116.0, 9.0, 20.0, 0.4, 8.0, 1.8),
    ("kichererbsen", "Kichererbsen, gekocht", "g", 150.0, "1 Portion", 164.0, 8.9, 27.0, 2.6, 7.6, 4.8),
    ("kidneybohnen", "Kidneybohnen, gekocht", "g", 150.0, "1 Portion", 127.0, 8.7, 22.8, 0.5, 6.4, 0.3),
    ("edamame", "Edamame", "g", 100.0, "1 Portion", 121.0, 11.9, 8.9, 5.2, 5.2, 2.2),
    ("hummus", "Hummus", "g", 50.0, "1 Portion", 250.0, 7.5, 14.0, 18.0, 6.0, 0.5),
    ("proteinpulver", "Proteinpulver (Whey)", "g", 30.0, "1 Messlöffel", 380.0, 78.0, 6.0, 5.0, 0.0, 4.0),
    // Gemüse
    ("broccoli", "Broccoli", "g", 200.0, "1 Portion", 34.0, 2.8, 4.0, 0.4, 2.6, 1.7),
    ("blumenkohl", "Blumenkohl", "g", 200.0, "1 Portion", 25.0, 1.9, 3.0, 0.3, 2.0, 1.9),
    ("karotte", "Karotte", "g", 80.0, "1 Stück", 41.0, 0.9, 7.5, 0.2, 2.8, 4.7),
    ("tomate", "Tomate", "g", 120.0, "1 Stück", 18.0, 0.9, 2.6, 0.2, 1.2, 2.6),
    ("gurke", "Gurke", "g", 150.0, "1/2 Gurke", 15.0, 0.6, 2.2, 0.1, 0.5, 1.7),
    ("peperoni", "Peperoni (Paprika)", "g", 150.0, "1 Stück", 31.0, 1.0, 6.0, 0.3, 2.1, 4.2),
    ("zucchetti", "Zucchetti", "g", 200.0, "1 Stück", 17.0, 1.2, 2.2, 0.3, 1.0, 1.7),
    ("spinat", "Spinat", "g", 100.0, "1 Portion", 23.0, 2.9, 1.4, 0.4, 2.2, 0.4),
    ("salat", "Blattsalat", "g", 80.0, "1 Portion", 15.0, 1.4, 1.5, 0.2, 1.3, 0.8),
    ("zwiebel", "Zwiebel", "g", 80.0, "1 Stück", 40.0, 1.1, 8.0, 0.1, 1.7, 4.2),
    ("champignons", "Champignons", "g", 100.0, "1 Portion", 22.0, 3.1, 0.6, 0.3, 1.0, 0.3),
    ("mais", "Mais (Dose)", "g", 100.0, "1 Portion", 86.0, 3.0, 15.0, 1.2, 2.5, 4.5),
    ("erbsen", "Erbsen (TK)", "g", 100.0, "1 Portion", 81.0, 5.4, 12.0, 0.4, 5.1, 5.7),
    ("gruene-bohnen", "Grüne Bohnen", "g", 150.0, "1 Portion", 31.0, 1.8, 4.0, 0.2, 2.7, 1.4),
    ("avocado", "Avocado", "g", 150.0, "1 Stück (ohne Kern)", 160.0, 2.0, 1.8, 15.0, 6.7, 0.7),
    // Obst
    ("apfel", "Apfel", "g", 150.0, "1 Stück", 52.0, 0.3, 12.0, 0.2, 2.4, 10.0),
    ("banane", "Banane", "g", 120.0, "1 Stück (ohne Schale)", 89.0, 1.1, 20.0, 0.3, 2.6, 12.0),
    ("birne", "Birne", "g", 160.0, "1 Stück", 57.0, 0.4, 12.5, 0.1, 3.1, 10.0),
    ("orange", "Orange", "g", 150.0, "1 Stück", 47.0, 0.9, 9.0, 0.1, 2.4, 9.0),
    ("mandarine", "Mandarine", "g", 70.0, "1 Stück", 53.0, 0.8, 11.0, 0.3, 1.8, 10.6),
    ("erdbeeren", "Erdbeeren", "g", 150.0, "1 Schale", 32.0, 0.7, 5.5, 0.3, 2.0, 4.9),
    ("heidelbeeren", "Heidelbeeren", "g", 125.0, "1 Schale", 57.0, 0.7, 12.0, 0.3, 2.4, 10.0),
    ("himbeeren", "Himbeeren", "g", 125.0, "1 Schale", 52.0, 1.2, 5.5, 0.7, 6.5, 4.4),
    ("trauben", "Weintrauben", "g", 150.0, "1 Portion", 69.0, 0.7, 16.0, 0.2, 0.9, 16.0),
    ("kiwi", "Kiwi", "g", 75.0, "1 Stück", 61.0, 1.1, 11.0, 0.5, 3.0, 9.0),
    ("ananas", "Ananas", "g", 150.0, "1 Portion", 50.0, 0.5, 12.0, 0.1, 1.4, 10.0),
    ("mango", "Mango", "g", 200.0, "1/2 Mango", 60.0, 0.8, 13.0, 0.4, 1.6, 13.7),
    ("wassermelone", "Wassermelone", "g", 300.0, "1 Schnitz", 30.0, 0.6, 7.5, 0.2, 0.4, 6.2),
    ("datteln", "Datteln, getrocknet", "g", 30.0, "3 Stück", 282.0, 2.5, 64.0, 0.4, 8.0, 63.0),
    ("rosinen", "Rosinen", "g", 30.0, "1 Handvoll", 299.0, 3.1, 79.0, 0.5, 3.7, 59.0),
    // Nüsse, Samen, Fette
    ("mandeln", "Mandeln", "g", 30.0, "1 Handvoll", 579.0, 21.0, 9.0, 50.0, 12.5, 4.4),
    ("baumnuesse", "Baumnüsse", "g", 30.0, "1 Handvoll", 654.0, 15.0, 7.0, 65.0, 6.7, 2.6),
    ("cashew", "Cashewkerne", "g", 30.0, "1 Handvoll", 553.0, 18.0, 27.0, 44.0, 3.3, 6.0),
    ("erdnuesse", "Erdnüsse", "g", 30.0, "1 Handvoll", 567.0, 26.0, 8.0, 49.0, 8.5, 4.0),
    ("erdnussbutter", "Erdnussbutter", "g", 15.0, "1 EL", 588.0, 25.0, 12.0, 50.0, 6.0, 6.0),
    ("chiasamen", "Chiasamen", "g", 15.0, "1 EL", 486.0, 17.0, 8.0, 31.0, 34.0, 0.0),
    ("leinsamen", "Leinsamen", "g", 10.0, "1 EL", 534.0, 18.0, 1.6, 42.0, 27.0, 1.6),
    ("olivenoel", "Olivenöl", "ml", 10.0, "1 EL", 884.0, 0.0, 0.0, 100.0, 0.0, 0.0),
    ("rapsoel", "Rapsöl", "ml", 10.0, "1 EL", 884.0, 0.0, 0.0, 100.0, 0.0, 0.0),
    // Süsses & Snacks
    ("schokolade-milch", "Milchschokolade", "g", 25.0, "1 Riegel", 535.0, 7.5, 57.0, 30.0, 2.0, 55.0),
    ("schokolade-dunkel", "Dunkle Schokolade 70 %", "g", 20.0, "2 Reihen", 598.0, 7.8, 34.0, 43.0, 11.0, 24.0),
    ("honig", "Honig", "g", 20.0, "1 EL", 304.0, 0.3, 82.0, 0.0, 0.2, 82.0),
    ("konfituere", "Konfitüre", "g", 20.0, "1 EL", 250.0, 0.4, 60.0, 0.1, 1.0, 55.0),
    ("zucker", "Zucker", "g", 5.0, "1 TL", 400.0, 0.0, 100.0, 0.0, 0.0, 100.0),
    ("chips", "Chips", "g", 30.0, "1 Handvoll", 536.0, 6.0, 53.0, 34.0, 4.5, 0.5),
    ("glace", "Glace (Vanille)", "g", 70.0, "1 Kugel", 207.0, 3.5, 24.0, 11.0, 0.7, 21.0),
    ("muesliriegel", "Müesliriegel", "g", 25.0, "1 Riegel", 420.0, 6.0, 65.0, 14.0, 5.0, 28.0),
    // Getränke
    ("kaffee", "Kaffee, schwarz", "ml", 150.0, "1 Tasse", 2.0, 0.1, 0.0, 0.0, 0.0, 0.0),
    ("cappuccino", "Cappuccino", "ml", 200.0, "1 Tasse", 40.0, 2.2, 3.3, 2.0, 0.0, 3.3),
    ("orangensaft", "Orangensaft", "ml", 200.0, "1 Glas", 45.0, 0.7, 10.0, 0.2, 0.2, 8.4),
    ("cola", "Cola", "ml", 330.0, "1 Dose", 42.0, 0.0, 10.6, 0.0, 0.0, 10.6),
    ("bier", "Bier", "ml", 330.0, "1 Flasche", 43.0, 0.5, 3.6, 0.0, 0.0, 0.0),
    ("rotwein", "Rotwein", "ml", 150.0, "1 Glas", 85.0, 0.1, 2.6, 0.0, 0.0, 0.6),
    // Saucen
    ("ketchup", "Ketchup", "g", 15.0, "1 EL", 112.0, 1.2, 26.0, 0.2, 0.3, 22.0),
    ("mayonnaise", "Mayonnaise", "g", 15.0, "1 EL", 680.0, 1.0, 1.5, 75.0, 0.0, 1.3),
    ("pesto", "Pesto alla Genovese", "g", 30.0, "2 EL", 450.0, 5.0, 6.0, 45.0, 2.0, 2.0),
    ("tomatensauce", "Tomatensauce", "g", 125.0, "1 Portion", 50.0, 1.5, 7.0, 1.5, 1.5, 5.5),
];

pub async fn seed(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = now_ts();
    let mut tx = pool.begin().await?;
    for (id, name, unit, serving, sname, kcal, p, c, f, fib, sug) in FOODS {
        sqlx::query(
            "INSERT OR IGNORE INTO foods (id, profile_id, name, unit, serving_size, serving_name, kcal, protein, carbs, fat,
                                          fiber, sugar, source, created_at)
             VALUES (?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'basis', ?)",
        )
        .bind(format!("basis-{id}"))
        .bind(name)
        .bind(unit)
        .bind(serving)
        .bind(sname)
        .bind(kcal)
        .bind(p)
        .bind(c)
        .bind(f)
        .bind(fib)
        .bind(sug)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}
