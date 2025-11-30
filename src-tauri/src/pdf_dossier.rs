// PDF Dossier Generator for Passenger Travel Reports
// Generates comprehensive travel reports with statistics, maps, and flight logs

use crate::database::Database;
use anyhow::{Result, Context};
use genpdf::{Document, Element as _, SimplePageDecorator};
use genpdf::elements::{Paragraph, TableLayout};
use genpdf::fonts;
use genpdf::style::Style;
use std::path::Path;

pub struct PassengerDossier {
    pub passenger_name: String,
    pub total_flights: usize,
    pub total_distance_km: f64,
    pub total_co2_kg: f64,
    pub unique_airports: usize,
    pub unique_countries: usize,
    pub flights: Vec<FlightRecord>,
    pub airport_visits: Vec<AirportVisit>,
}

pub struct FlightRecord {
    pub flight_number: Option<String>,
    pub date: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub distance_km: f64,
    pub co2_kg: f64,
    pub aircraft_type: Option<String>,
}

pub struct AirportVisit {
    pub airport_code: String,
    pub visit_count: i32,
}

impl PassengerDossier {
    pub fn from_passenger(db: &Database, passenger_name: &str, user_id: &str) -> Result<Self> {
        // Get all flights for this passenger
        let flights_query = "
            SELECT
                f.flight_number,
                f.departure_datetime,
                f.departure_airport,
                f.arrival_airport,
                f.distance_km,
                f.co2_emissions_kg,
                f.aircraft_type
            FROM flights f
            INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
            WHERE pm.passenger_name = ?1 AND f.user_id = ?2
            ORDER BY f.departure_datetime DESC
        ";

        let mut stmt = db.conn.prepare(flights_query)?;
        let flight_records: Vec<FlightRecord> = stmt
            .query_map([passenger_name, user_id], |row| {
                Ok(FlightRecord {
                    flight_number: row.get(0)?,
                    date: row.get::<_, String>(1)?.split('T').next().unwrap_or("").to_string(),
                    departure_airport: row.get(2)?,
                    arrival_airport: row.get(3)?,
                    distance_km: row.get(4)?,
                    co2_kg: row.get(5)?,
                    aircraft_type: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Calculate statistics
        let total_flights = flight_records.len();
        let total_distance_km: f64 = flight_records.iter().map(|f| f.distance_km).sum();
        let total_co2_kg: f64 = flight_records.iter().map(|f| f.co2_kg).sum();

        // Count unique airports
        let mut unique_airports_set = std::collections::HashSet::new();
        for flight in &flight_records {
            unique_airports_set.insert(&flight.departure_airport);
            unique_airports_set.insert(&flight.arrival_airport);
        }
        let unique_airports = unique_airports_set.len();

        // Count unique countries (from airports)
        let countries_query = "
            SELECT DISTINCT a.country
            FROM airports a
            WHERE a.iata_code IN (
                SELECT DISTINCT f.departure_airport FROM flights f
                INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
                WHERE pm.passenger_name = ?1 AND f.user_id = ?2
                UNION
                SELECT DISTINCT f.arrival_airport FROM flights f
                INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
                WHERE pm.passenger_name = ?1 AND f.user_id = ?2
            )
            AND a.country IS NOT NULL
        ";

        let unique_countries: usize = db.conn
            .prepare(countries_query)?
            .query_map([passenger_name, user_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .len();

        // Get airport visit counts
        let airports_query = "
            SELECT airport_code, COUNT(*) as visit_count
            FROM (
                SELECT f.departure_airport as airport_code
                FROM flights f
                INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
                WHERE pm.passenger_name = ?1 AND f.user_id = ?2
                UNION ALL
                SELECT f.arrival_airport as airport_code
                FROM flights f
                INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
                WHERE pm.passenger_name = ?1 AND f.user_id = ?2
            )
            GROUP BY airport_code
            ORDER BY visit_count DESC
            LIMIT 10
        ";

        let mut stmt = db.conn.prepare(airports_query)?;
        let airport_visits: Vec<AirportVisit> = stmt
            .query_map([passenger_name, user_id], |row| {
                Ok(AirportVisit {
                    airport_code: row.get(0)?,
                    visit_count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            passenger_name: passenger_name.to_string(),
            total_flights,
            total_distance_km,
            total_co2_kg,
            unique_airports,
            unique_countries,
            flights: flight_records,
            airport_visits,
        })
    }

    pub fn generate_pdf(&self, output_path: &Path) -> Result<()> {
        // Load font - try platform-specific paths first, then fall back
        let default_font = fonts::from_files("./fonts", "LiberationSans", None)
            // Linux paths
            .or_else(|_| fonts::from_files("/usr/share/fonts/liberation", "LiberationSans", None))
            .or_else(|_| fonts::from_files("/usr/share/fonts/truetype/liberation", "LiberationSans-Regular", None))
            .or_else(|_| fonts::from_files("/usr/share/fonts/truetype/dejavu", "DejaVuSans", None))
            // macOS paths
            .or_else(|_| fonts::from_files("/System/Library/Fonts", "Helvetica", None))
            .or_else(|_| fonts::from_files("/Library/Fonts", "Arial", None))
            .or_else(|_| fonts::from_files("/System/Library/Fonts/Supplemental", "Arial", None))
            // Windows paths
            .or_else(|_| fonts::from_files("C:\\Windows\\Fonts", "arial", None))
            .or_else(|_| fonts::from_files("C:\\Windows\\Fonts", "calibri", None))
            // User home directory fonts (cross-platform)
            .or_else(|e| {
                if let Some(home) = dirs::font_dir() {
                    fonts::from_files(&home, "Arial", None)
                        .or_else(|_| fonts::from_files(&home, "LiberationSans", None))
                } else {
                    // Propagate the previous genpdf error
                    Err(e)
                }
            })
            .unwrap_or_else(|_| {
                // Fallback to built-in font
                genpdf::fonts::FontFamily {
                    regular: genpdf::fonts::FontData::new(vec![], None).unwrap(),
                    bold: genpdf::fonts::FontData::new(vec![], None).unwrap(),
                    italic: genpdf::fonts::FontData::new(vec![], None).unwrap(),
                    bold_italic: genpdf::fonts::FontData::new(vec![], None).unwrap(),
                }
            });

        let mut doc = Document::new(default_font);
        doc.set_title(format!("Travel Dossier - {}", self.passenger_name));

        // Add page decorator (header/footer)
        let mut decorator = SimplePageDecorator::new();
        decorator.set_margins(10);
        doc.set_page_decorator(decorator);

        // Cover Page
        self.add_cover_page(&mut doc)?;

        // Statistics Summary
        self.add_statistics_page(&mut doc)?;

        // Top Airports
        self.add_airports_page(&mut doc)?;

        // Detailed Flight Log
        self.add_flight_log(&mut doc)?;

        // Render to file
        doc.render_to_file(output_path)
            .context("Failed to render PDF")?;

        Ok(())
    }

    fn add_cover_page(&self, doc: &mut Document) -> Result<()> {
        doc.push(
            Paragraph::new("")
                .padded(genpdf::Margins::vh(50, 0))
        );

        doc.push(
            Paragraph::new("PASSENGER TRAVEL DOSSIER")
                .aligned(genpdf::Alignment::Center)
                .styled(Style::new().bold().with_font_size(24))
        );

        doc.push(
            Paragraph::new("")
                .padded(genpdf::Margins::vh(10, 0))
        );

        doc.push(
            Paragraph::new(&self.passenger_name)
                .aligned(genpdf::Alignment::Center)
                .styled(Style::new().with_font_size(18))
        );

        doc.push(
            Paragraph::new("")
                .padded(genpdf::Margins::vh(20, 0))
        );

        // Quick stats box
        let stats_text = format!(
            "Total Flights: {}\nTotal Distance: {:.0} km ({:.0} miles)\nTotal CO2 Emissions: {:.0} kg\nUnique Airports: {}\nCountries Visited: {}",
            self.total_flights,
            self.total_distance_km,
            self.total_distance_km * 0.621371,
            self.total_co2_kg,
            self.unique_airports,
            self.unique_countries
        );

        doc.push(
            Paragraph::new(stats_text)
                .aligned(genpdf::Alignment::Center)
                .styled(Style::new().with_font_size(12))
        );

        doc.push(genpdf::elements::PageBreak::new());

        Ok(())
    }

    fn add_statistics_page(&self, doc: &mut Document) -> Result<()> {
        doc.push(
            Paragraph::new("TRAVEL STATISTICS")
                .styled(Style::new().bold().with_font_size(18))
        );

        doc.push(Paragraph::new("").padded(genpdf::Margins::vh(5, 0)));

        // Environmental Impact
        doc.push(
            Paragraph::new("Environmental Impact")
                .styled(Style::new().bold().with_font_size(14))
        );

        let co2_trees = (self.total_co2_kg / 21.77).ceil(); // Trees needed to offset for 1 year
        let co2_comparison = format!(
            "Total CO2 Emissions: {:.2} kg\n\
             Equivalent to:\n\
             - {:.0} trees needed to offset for 1 year\n\
             - {:.0} km driven by average car\n\
             - {:.2} tons CO2",
            self.total_co2_kg,
            co2_trees,
            self.total_co2_kg * 5.0, // Rough car equivalent
            self.total_co2_kg / 1000.0
        );

        doc.push(Paragraph::new(co2_comparison).styled(Style::new().with_font_size(11)));

        doc.push(Paragraph::new("").padded(genpdf::Margins::vh(10, 0)));

        // Distance Milestones
        doc.push(
            Paragraph::new("Distance Milestones")
                .styled(Style::new().bold().with_font_size(14))
        );

        let earth_circumference = 40075.0;
        let moon_distance = 384400.0;
        let times_around_earth = self.total_distance_km / earth_circumference;

        let distance_comparison = format!(
            "Total Distance: {:.0} km ({:.0} miles)\n\
             Equivalent to:\n\
             - {:.2} times around the Earth\n\
             - {:.2}% of the distance to the Moon",
            self.total_distance_km,
            self.total_distance_km * 0.621371,
            times_around_earth,
            (self.total_distance_km / moon_distance) * 100.0
        );

        doc.push(Paragraph::new(distance_comparison).styled(Style::new().with_font_size(11)));

        doc.push(genpdf::elements::PageBreak::new());

        Ok(())
    }

    fn add_airports_page(&self, doc: &mut Document) -> Result<()> {
        doc.push(
            Paragraph::new("TOP AIRPORTS")
                .styled(Style::new().bold().with_font_size(18))
        );

        doc.push(Paragraph::new("").padded(genpdf::Margins::vh(5, 0)));

        doc.push(
            Paragraph::new("Most Frequently Visited Airports")
                .styled(Style::new().bold().with_font_size(14))
        );

        for (idx, airport) in self.airport_visits.iter().enumerate() {
            let text = format!("{}. {} - {} visits", idx + 1, airport.airport_code, airport.visit_count);
            doc.push(Paragraph::new(text).styled(Style::new().with_font_size(11)));
        }

        doc.push(genpdf::elements::PageBreak::new());

        Ok(())
    }

    fn add_flight_log(&self, doc: &mut Document) -> Result<()> {
        doc.push(
            Paragraph::new("DETAILED FLIGHT LOG")
                .styled(Style::new().bold().with_font_size(18))
        );

        doc.push(Paragraph::new("").padded(genpdf::Margins::vh(5, 0)));

        // Create table
        let mut table = TableLayout::new(vec![1, 2, 1, 1, 1, 1]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

        // Header row
        table.row()
            .element(Paragraph::new("Date").styled(Style::new().bold()))
            .element(Paragraph::new("Flight").styled(Style::new().bold()))
            .element(Paragraph::new("From").styled(Style::new().bold()))
            .element(Paragraph::new("To").styled(Style::new().bold()))
            .element(Paragraph::new("Distance").styled(Style::new().bold()))
            .element(Paragraph::new("CO2").styled(Style::new().bold()))
            .push()?;

        // Data rows
        for flight in &self.flights {
            table.row()
                .element(Paragraph::new(&flight.date))
                .element(Paragraph::new(flight.flight_number.as_ref().unwrap_or(&"-".to_string())))
                .element(Paragraph::new(&flight.departure_airport))
                .element(Paragraph::new(&flight.arrival_airport))
                .element(Paragraph::new(format!("{:.0} km", flight.distance_km)))
                .element(Paragraph::new(format!("{:.1} kg", flight.co2_kg)))
                .push()?;
        }

        doc.push(table);

        Ok(())
    }
}
