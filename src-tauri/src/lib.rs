// Flight Tracker Pro - Main application module

mod agent_memory;
mod agent_server;
mod agent_tracking;
mod calculations;
mod commands;
mod database;
mod deepseek;
mod doc_ingestion;
mod doc_worker;
pub mod extract;
mod gemini;
mod geo;
mod grok;
mod investigation;
mod models;
mod ocr;
mod ocr_learning;
mod pdf_dossier;
mod workflow;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize database
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            // Create app directory if it doesn't exist
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");

            let db_path = app_dir.join("flight_tracker.db");
            let database = database::Database::new(db_path.clone()).expect("Failed to initialize database");

            // Store database in app state
            app.manage(commands::AppState {
                db: Mutex::new(database),
            });

            // Initialize workflow state
            app.manage(commands::workflow::WorkflowState::new(app.handle().clone()));

            // Spawn WebSocket agent server on port 9528 for bridge integration
            let server_db_path = db_path.clone();
            tauri::async_runtime::spawn(async move {
                let server = agent_server::AgentServer::new(9528, server_db_path);
                if let Err(e) = server.start().await {
                    eprintln!("❌ Agent server failed to start: {}", e);
                } else {
                    println!("✅ Agent server started successfully");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings
            commands::get_setting,
            commands::set_setting,
            // Users
            commands::create_user,
            commands::get_user,
            commands::get_primary_user,
            commands::list_all_users,
            commands::update_user_name,
            // Flights
            commands::create_flight,
            commands::get_flight,
            commands::list_flights,
            commands::delete_flight,
            // CSV Import
            commands::preview_csv_import,
            commands::import_flights_from_csv_with_mapping,
            commands::import_flights_from_csv,
            commands::preload_test_data_batch,
            // Statistics
            commands::get_statistics,
            commands::get_airport_list,
            commands::get_analytics,
            // Distance & CO2 Calculations
            commands::calculate_distance,
            commands::calculate_co2_emissions,
            commands::calculate_per_passenger_co2,
            commands::calculate_flight_duration,
            commands::fuel_kg_to_liters,
            commands::fuel_kg_to_gallons,
            // OCR
            commands::analyze_boarding_pass,
            commands::batch_analyze_boarding_passes,
            // OCR Learning
            commands::record_ocr_correction,
            commands::get_ocr_suggestions,
            commands::accept_ocr_suggestion,
            commands::reject_ocr_suggestion,
            commands::get_ocr_correction_history,
            commands::get_active_learning_patterns,
            // Data Management
            commands::export_data_to_csv,
            commands::reset_database,
            // Investigations
            commands::investigate_flight,
            commands::get_flight_investigation,
            commands::list_all_investigations,
            // Passenger Management
            commands::get_all_passenger_names,
            commands::save_passenger_mapping,
            commands::delete_passenger_mapping,
            commands::get_all_passenger_mappings,
            commands::get_passenger_details,
            commands::export_passenger_dossier,
            commands::get_canonical_names,
            commands::get_unmapped_passengers,
            commands::get_aliases_for_canonical,
            // Identity Fusion (Canonical Architecture)
            commands::bootstrap_identities,
            commands::bootstrap_identities_batch,
            commands::merge_alias,
            commands::list_canonical_passengers,
            commands::get_passenger_aliases,
            commands::list_unmerged_passengers,
            commands::rename_canonical_passenger,
            commands::get_identity_stats,
            // Do Not Deduplicate Management
            commands::mark_no_dedup,
            commands::unmark_no_dedup,
            commands::is_no_dedup,
            commands::list_no_dedup_passengers,
            // Delete Passenger
            commands::delete_passenger,
            // Search with Aliases
            commands::search_passengers,
            // Aggregated Passenger Details (alias-aware)
            commands::get_passenger_details_aggregated,
            // Split Passenger
            commands::preview_split_passenger,
            commands::split_passenger,
            commands::detect_compound_name,
            // Batch Split
            commands::find_splittable_passengers,
            commands::batch_split_passengers,
            // DeepSeek Research
            commands::research_flight_with_deepseek,
            // Grok Research
            commands::research_flight_with_grok,
            commands::multi_provider_flight_research,
            // Gemini Chat
            commands::chat_with_gemini,
            // DeepSeek Chat
            commands::chat_with_deepseek,
            // Research Reports
            commands::save_research_report,
            commands::get_research_report,
            commands::list_research_reports,
            commands::delete_research_report,
            commands::count_research_reports,
            commands::export_research_report_to_markdown,
            // Journey Management
            commands::create_journey,
            commands::get_journey,
            commands::list_user_journeys,
            commands::update_journey,
            commands::delete_journey,
            commands::add_flight_to_journey,
            commands::remove_flight_from_journey,
            commands::get_journey_flights,
            // Pilot Logbook
            commands::create_pilot_logbook_entry,
            commands::get_pilot_logbook_entry,
            commands::get_pilot_logbook_by_flight,
            commands::list_all_pilot_logbook_entries,
            commands::update_pilot_logbook_entry,
            commands::delete_pilot_logbook_entry,
            commands::get_pilot_logbook_totals,
            // Frequent Flyer Programs
            commands::create_ffp,
            commands::get_ffp,
            commands::list_user_ffps,
            commands::update_ffp,
            commands::delete_ffp,
            // Airports
            commands::create_airport,
            commands::get_airport,
            commands::list_all_airports,
            commands::update_airport,
            commands::delete_airport,
            // Airport Enrichment & Search
            commands::enrich_airport_data,
            commands::get_missing_coordinates_count,
            commands::get_total_airports_count,
            commands::lookup_airport,
            commands::search_airports_csv,
            commands::get_csv_airport_count,
            commands::get_all_csv_airports,
            commands::get_csv_airports_by_codes,
            commands::get_visited_airport_codes,
            commands::import_visited_airports,
            // Aircraft Types
            commands::create_aircraft_type,
            commands::get_aircraft_type,
            commands::list_all_aircraft_types,
            commands::update_aircraft_type,
            commands::delete_aircraft_type,
            // Custom Documents
            commands::create_custom_document,
            commands::get_custom_document,
            commands::list_user_documents,
            commands::update_custom_document,
            commands::delete_custom_document,
            commands::import_document_with_ocr,
            // Fuel Prices (Basic CRUD)
            commands::create_fuel_price,
            commands::get_fuel_price,
            commands::list_fuel_prices,
            commands::get_fuel_price_for_date,
            commands::update_fuel_price,
            commands::delete_fuel_price,
            commands::calculate_fuel_cost,
            // Fuel Tracking (AI-powered search & user entries)
            commands::search_fuel_prices,
            commands::get_cached_fuel_prices,
            commands::add_fuel_entry,
            commands::get_fuel_entries,
            commands::get_fuel_stats,
            commands::delete_fuel_entry,
            commands::get_fuel_types,
            commands::add_fuel_type,
            commands::delete_fuel_type,
            // Analytics
            commands::get_temporal_analysis,
            commands::get_geospatial_analysis,
            commands::get_passenger_network,
            commands::get_comparative_metrics,
            commands::get_aircraft_utilization,
            commands::get_cost_breakdown,
            commands::get_day_night_stats,
            commands::get_long_haul_flights,
            commands::get_pilot_currency,
            commands::get_monthly_cost_trend,
            commands::get_runway_risk_data,
            // Network Scanner (Physical Security)
            commands::scan_wifi_networks,
            commands::scan_bluetooth_devices,
            commands::scan_local_network,
            commands::trust_wifi_device,
            commands::trust_bluetooth_device,
            commands::untrust_wifi_device,
            commands::untrust_bluetooth_device,
            // Agent Memory
            commands::search_agent_memories,
            commands::get_flight_memories,
            commands::get_recent_memories,
            commands::get_memory_stats,
            commands::cleanup_expired_memories,
            // Document Ingestion
            commands::enqueue_pdf_for_processing,
            commands::get_ingestion_queue_stats,
            commands::recover_ingestion_queue,
            commands::start_document_worker,
            commands::query_relationship_graph,
            commands::build_flight_relationships,
            commands::get_relationship_stats,
            // Custom Schema Builder
            commands::create_custom_schema,
            commands::list_custom_schemas,
            commands::get_schema_fields,
            commands::delete_custom_schema,
            commands::create_custom_record,
            commands::list_custom_records,
            commands::update_custom_record,
            commands::delete_custom_record,
            commands::set_flight_custom_field,
            commands::get_flight_custom_fields,
            // Self-Improvement System
            commands::record_correction,
            commands::get_suggested_correction,
            commands::record_user_pattern,
            commands::get_smart_defaults,
            commands::detect_flight_anomalies,
            commands::get_pending_anomalies,
            commands::resolve_anomaly,
            commands::find_duplicates,
            commands::get_pending_duplicates,
            commands::update_route_statistics,
            commands::get_route_prediction,
            commands::save_fuel_price,
            commands::get_fuel_price_history,
            commands::cache_ai_response,
            commands::get_cached_ai_response,
            commands::get_self_improvement_stats,
            commands::populate_route_statistics,
            // Workflow
            commands::validate_workflow,
            commands::get_execution_order,
            commands::execute_workflow,
            commands::is_workflow_running,
            commands::cancel_workflow,
            commands::export_workflow,
            commands::import_workflow,
            // Workflow Checkpoints
            commands::init_workflow_checkpoint,
            commands::create_checkpoint,
            commands::get_checkpoint_history,
            commands::get_checkpoint_state,
            // AI Workflow Generation
            commands::generate_workflow_ai,
            // Network Sentinel
            commands::get_network_stats,
            commands::get_network_flows,
            commands::get_network_anomalies,
            commands::query_network_history,
            commands::detect_location_from_ip,
            // Data Editor
            commands::find_duplicate_flights,
            commands::merge_duplicate_flights,
            commands::update_flight,
            commands::bulk_delete_flights,
            commands::get_flights_for_editor,
            commands::get_flight_count,
            commands::get_data_editor_stats,
            commands::remove_passenger_from_flights,
            commands::rename_passenger_in_flights,
            // Media Gallery
            commands::upload_media_file,
            commands::list_media_files,
            commands::get_media_file,
            commands::get_media_file_path,
            commands::update_media_file,
            commands::toggle_media_favorite,
            commands::delete_media_file,
            commands::get_media_stats,
            commands::get_flight_media,
            commands::get_journey_media,
            // Batch Calculations (Distance & CO2)
            commands::batch_calculate_missing_distances,
            commands::batch_recalculate_co2,
            commands::batch_calculate_all,
            commands::batch_calculate_streaming,
            commands::get_calculation_stats,
            // Active Defense (D-Bus Sentinel Controls)
            commands::get_all_sentinel_metrics,
            commands::get_cpu_snapshot,
            commands::cpu_set_governor,
            commands::cpu_disable_turbo,
            commands::cpu_enable_turbo,
            commands::cpu_emergency_power_reduce,
            commands::cpu_reset_controls,
            commands::get_memory_snapshot,
            commands::memory_drop_caches,
            commands::memory_emergency_relief,
            commands::memory_trigger_oom_kill,
            commands::memory_compact,
            commands::get_thermal_snapshot,
            commands::thermal_emergency_cool,
            commands::thermal_set_power_limit,
            commands::thermal_reset_controls,
            commands::get_gpu_snapshot,
            commands::gpu_emergency_throttle,
            commands::gpu_set_power_limit,
            commands::gpu_reset,
            commands::gpu_kill_process,
            commands::get_network_snapshot,
            commands::network_block_ip,
            commands::network_unblock_ip,
            commands::network_kill_connections_ip,
            commands::network_rate_limit_ip,
            commands::network_block_process,
            commands::network_clear_all_blocks,
            commands::process_freeze,
            commands::process_thaw,
            commands::process_kill,
            commands::process_set_nice,
            commands::process_reap_zombies,
            commands::process_get_top_cpu,
            commands::process_get_top_memory,
            commands::emergency_all_systems,
            commands::lockdown_network,
            commands::performance_mode,
            commands::reset_all_controls,
            // Network Tools (IP investigation)
            commands::network_whois,
            commands::network_nslookup,
            commands::network_ping,
            commands::network_traceroute,
            commands::network_geoip,
            // Note: IP blocking (network_block_ip, network_unblock_ip) is in Active Defense section above
            // Donation & Support
            commands::generate_qr_code,
            commands::generate_qr_code_themed,
            commands::get_donation_config,
            commands::record_donation_click,
            // AI Models
            commands::get_ai_models,
            commands::get_models_by_provider,
            // Initialization
            commands::initialize_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
