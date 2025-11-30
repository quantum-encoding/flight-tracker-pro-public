// Geographic calculations for flight distances

use std::f64::consts::PI;

// Airport coordinates database (subset of major airports)
// Format: IATA code -> (latitude, longitude)
pub fn get_airport_coords(iata: &str) -> Option<(f64, f64)> {
    match iata.to_uppercase().as_str() {
        // Major US Hubs
        "JFK" => Some((40.6413, -73.7781)),
        "LAX" => Some((33.9416, -118.4085)),
        "ORD" => Some((41.9742, -87.9073)),
        "SFO" => Some((37.6213, -122.3790)),
        "MIA" => Some((25.7959, -80.2870)),
        "ATL" => Some((33.6407, -84.4277)),
        "DFW" => Some((32.8998, -97.0403)),
        "DEN" => Some((39.8561, -104.6737)),
        "SEA" => Some((47.4502, -122.3088)),
        "LAS" => Some((36.0840, -115.1537)),
        "PHX" => Some((33.4352, -112.0101)),
        "IAH" => Some((29.9902, -95.3368)),
        "MCO" => Some((28.4312, -81.3081)),
        "EWR" => Some((40.6895, -74.1745)),
        "BOS" => Some((42.3656, -71.0096)),
        "CLT" => Some((35.2144, -80.9473)),
        "MSP" => Some((44.8848, -93.2223)),
        "DTW" => Some((42.2124, -83.3534)),
        "PHL" => Some((39.8729, -75.2437)),

        // New York Metro
        "LGA" => Some((40.7769, -73.8740)),
        "TEB" => Some((40.8501, -74.0608)), // Teterboro
        "HPN" => Some((41.0670, -73.7076)), // White Plains
        "ISP" => Some((40.7952, -73.1002)), // Long Island

        // Florida
        "PBI" => Some((26.6832, -80.0956)), // Palm Beach
        "FLL" => Some((26.0742, -80.1506)), // Fort Lauderdale
        "TPA" => Some((27.9755, -82.5332)), // Tampa
        "RSW" => Some((26.5362, -81.7552)), // Fort Myers
        "JAX" => Some((30.4941, -81.6879)), // Jacksonville
        "PNS" => Some((30.4734, -87.1866)), // Pensacola
        "SRQ" => Some((27.3954, -82.5544)), // Sarasota

        // Northeast
        "BWI" => Some((39.1774, -76.6684)), // Baltimore
        "DCA" => Some((38.8521, -77.0377)), // Washington National
        "IAD" => Some((38.9531, -77.4565)), // Dulles
        "BDL" => Some((41.9389, -72.6832)), // Hartford
        "PVD" => Some((41.7240, -71.4281)), // Providence
        "ALB" => Some((42.7483, -73.8017)), // Albany
        "BUF" => Some((42.9405, -78.7322)), // Buffalo
        "ROC" => Some((43.1189, -77.6724)), // Rochester
        "SYR" => Some((43.1112, -76.1063)), // Syracuse
        "ACK" => Some((41.2531, -70.0602)), // Nantucket
        "MVY" => Some((41.3932, -70.6143)), // Martha's Vineyard
        "RIC" => Some((37.5052, -77.3197)), // Richmond
        "ORF" => Some((36.8946, -76.2012)), // Norfolk
        "RDU" => Some((35.8801, -78.7880)), // Raleigh-Durham

        // Midwest
        "CMH" => Some((39.9980, -82.8919)), // Columbus
        "DAY" => Some((39.9024, -84.2194)), // Dayton
        "CVG" => Some((39.0488, -84.6678)), // Cincinnati
        "CLE" => Some((41.4117, -81.8498)), // Cleveland
        "IND" => Some((39.7173, -86.2944)), // Indianapolis
        "MKE" => Some((42.9472, -87.9065)), // Milwaukee
        "STL" => Some((38.7487, -90.3700)), // St. Louis
        "MCI" => Some((39.2976, -94.7139)), // Kansas City
        "OMA" => Some((41.3032, -95.8941)), // Omaha
        "DSM" => Some((41.5340, -93.6631)), // Des Moines

        // South
        "SAV" => Some((32.1276, -81.2021)), // Savannah
        "BHM" => Some((33.5629, -86.7535)), // Birmingham
        "MSY" => Some((29.9934, -90.2580)), // New Orleans
        "AUS" => Some((30.1945, -97.6699)), // Austin
        "SAT" => Some((29.5337, -98.4698)), // San Antonio
        "HOU" => Some((29.6454, -95.2789)), // Houston Hobby
        "DAL" => Some((32.8471, -96.8518)), // Dallas Love
        "OKC" => Some((35.3931, -97.6007)), // Oklahoma City
        "TUL" => Some((36.1984, -95.8881)), // Tulsa
        "MEM" => Some((35.0424, -89.9767)), // Memphis
        "BNA" => Some((36.1245, -86.6782)), // Nashville
        "LIT" => Some((34.7294, -92.2243)), // Little Rock

        // West
        "ABQ" => Some((35.0402, -106.6092)), // Albuquerque
        "ELP" => Some((31.8072, -106.3778)), // El Paso
        "TUS" => Some((32.1161, -110.9411)), // Tucson
        "SNA" => Some((33.6762, -117.8682)), // Orange County
        "ONT" => Some((34.0560, -117.6012)), // Ontario
        "BUR" => Some((34.2007, -118.3587)), // Burbank
        "SJC" => Some((37.3626, -121.9290)), // San Jose
        "OAK" => Some((37.7213, -122.2208)), // Oakland
        "SMF" => Some((38.6954, -121.5901)), // Sacramento
        "RNO" => Some((39.4991, -119.7681)), // Reno
        "BOI" => Some((43.5644, -116.2228)), // Boise
        "SLC" => Some((40.7899, -111.9791)), // Salt Lake City
        "PDX" => Some((45.5898, -122.5951)), // Portland
        "SBA" => Some((34.4264, -119.8403)), // Santa Barbara
        "PSP" => Some((33.8297, -116.5067)), // Palm Springs
        "SAN" => Some((32.7338, -117.1933)), // San Diego
        "TVC" => Some((44.7414, -85.5822)),  // Traverse City
        "ASE" => Some((39.2232, -106.8691)), // Aspen
        "MTJ" => Some((38.5098, -107.8938)), // Montrose
        "JAC" => Some((43.6073, -110.7377)), // Jackson Hole
        "SUN" => Some((43.5048, -114.2962)), // Sun Valley

        // Alaska & Hawaii
        "ANC" => Some((61.1743, -149.9962)), // Anchorage
        "FAI" => Some((64.8151, -147.8562)), // Fairbanks
        "HNL" => Some((21.3187, -157.9225)), // Honolulu
        "OGG" => Some((20.8986, -156.4305)), // Maui
        "KOA" => Some((19.7388, -156.0456)), // Kona
        "LIH" => Some((21.9760, -159.3389)), // Lihue

        // Caribbean
        "SJU" => Some((18.4394, -66.0018)), // San Juan
        "STT" => Some((18.3373, -64.9733)), // St. Thomas
        "STX" => Some((17.7019, -64.7986)), // St. Croix

        // Canada
        "YYZ" => Some((43.6777, -79.6248)),  // Toronto
        "YVR" => Some((49.1967, -123.1815)), // Vancouver
        "YUL" => Some((45.4707, -73.7408)),  // Montreal
        "YYC" => Some((51.1311, -114.0103)), // Calgary
        "YEG" => Some((53.3097, -113.5797)), // Edmonton

        // Mexico
        "MEX" => Some((19.4363, -99.0721)),  // Mexico City
        "CUN" => Some((21.0365, -86.8771)),  // Cancun
        "PVR" => Some((20.6801, -105.2544)), // Puerto Vallarta
        "CZM" => Some((20.5224, -86.9256)),  // Cozumel
        "SJD" => Some((23.1518, -109.7209)), // Cabo San Lucas

        // Europe
        "LHR" => Some((51.4700, -0.4543)),
        "CDG" => Some((49.0097, 2.5479)),
        "FRA" => Some((50.0379, 8.5622)),
        "AMS" => Some((52.3105, 4.7683)),
        "MAD" => Some((40.4983, -3.5676)),
        "FCO" => Some((41.8003, 12.2389)),
        "MUC" => Some((48.3537, 11.7750)),
        "ZRH" => Some((47.4582, 8.5481)),
        "VIE" => Some((48.1103, 16.5697)),
        "CPH" => Some((55.6180, 12.6508)),
        "LGW" => Some((51.1537, -0.1821)), // London Gatwick
        "MAN" => Some((53.3537, -2.2750)), // Manchester
        "EDI" => Some((55.9500, -3.3725)), // Edinburgh
        "DUB" => Some((53.4213, -6.2701)), // Dublin
        "LIS" => Some((38.7813, -9.1359)), // Lisbon
        "BCN" => Some((41.2974, 2.0833)),  // Barcelona
        "ATH" => Some((37.9364, 23.9445)), // Athens
        "IST" => Some((41.2753, 28.7519)), // Istanbul
        "NCE" => Some((43.6584, 7.2159)),  // Nice
        "GVA" => Some((46.2381, 6.1090)),  // Geneva

        // Asia
        "HND" => Some((35.5494, 139.7798)),
        "NRT" => Some((35.7647, 140.3863)),
        "HKG" => Some((22.3080, 113.9185)),
        "SIN" => Some((1.3644, 103.9915)),
        "ICN" => Some((37.4602, 126.4407)),
        "PEK" => Some((40.0799, 116.6031)),
        "PVG" => Some((31.1434, 121.8052)),
        "BKK" => Some((13.6900, 100.7501)),
        "DXB" => Some((25.2532, 55.3657)),
        "DEL" => Some((28.5562, 77.1000)), // Delhi
        "BOM" => Some((19.0896, 72.8656)), // Mumbai

        // Australia & Oceania
        "SYD" => Some((-33.9399, 151.1753)),
        "MEL" => Some((-37.6690, 144.8410)),
        "AKL" => Some((-37.0082, 174.7850)),
        "BNE" => Some((-27.3942, 153.1218)), // Brisbane
        "PER" => Some((-31.9403, 115.9669)), // Perth

        // South America
        "GRU" => Some((-23.4356, -46.4731)),
        "GIG" => Some((-22.8099, -43.2505)),
        "SCL" => Some((-33.3930, -70.7858)),
        "BOG" => Some((4.7016, -74.1469)),
        "LIM" => Some((-12.0219, -77.1143)),
        "EZE" => Some((-34.8222, -58.5358)), // Buenos Aires

        // Africa
        "CPT" => Some((-33.9690, 18.6021)),
        "JNB" => Some((-26.1392, 28.2460)),
        "CAI" => Some((30.1219, 31.4056)),

        _ => None,
    }
}

/// Calculate great circle distance between two coordinates using Haversine formula
/// Returns distance in both nautical miles and kilometers
pub fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
    let earth_radius_km = 6371.0;
    let earth_radius_nm = 3440.065; // nautical miles

    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lat = (lat2 - lat1) * PI / 180.0;
    let delta_lon = (lon2 - lon1) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    let distance_km = earth_radius_km * c;
    let distance_nm = earth_radius_nm * c;

    (distance_nm, distance_km)
}

/// Calculate distance between two airports by IATA code
pub fn calculate_airport_distance(from: &str, to: &str) -> Option<(f64, f64)> {
    let from_coords = get_airport_coords(from)?;
    let to_coords = get_airport_coords(to)?;

    Some(calculate_distance(
        from_coords.0,
        from_coords.1,
        to_coords.0,
        to_coords.1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jfk_to_lax() {
        let (nm, km) = calculate_airport_distance("JFK", "LAX").unwrap();
        // Actual distance is approximately 2475 miles / 3983 km
        assert!((nm - 2150.0).abs() < 100.0); // Within 100nm
        assert!((km - 3983.0).abs() < 200.0); // Within 200km
    }
}
