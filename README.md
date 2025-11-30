# Flight Tracker Pro

A forensic-grade, offline-first platform for transforming fragmented travel records into a powerful, interactive intelligence dashboard.

Built with Rust, Tauri 2.0, Svelte 5, and TailwindCSS, Flight Tracker Pro is designed for power users, researchers, and pilots who demand absolute data integrity and control.

## Overview

Go beyond simple logging to uncover hidden patterns, analyze complex travel networks, and cross-reference entities against open-source information. Flight Tracker Pro is a comprehensive desktop application that combines a robust, private flight database with a cutting-edge, multi-agent AI research engine.

It is a sovereign, uncensorable tool for objective, forensic, open-source intelligence (OSINT) analysis.

## Key Capabilities

### Forensic-Grade Database & Data Ingestion

- **Immutable Architecture**: Ingest and structure unlimited flight logs, manifests, and passenger data in a secure, local SQLite database.

- **Multi-Source Data Ingestion**: Process data from any source.
  - **Interactive CSV Onboarding**: A professional suite for cleaning, validating, and mapping legacy data from any CSV format. Features live preview, inline editing, and smart column detection.
  - **Bulk OCR for Scanned Documents**: Use the integrated Gemini Vision engine to perform optical character recognition on boarding passes and scanned manifests, with intelligent, self-healing retries for rate-limiting.

- **Manual Entry**: A fast, streamlined interface for manual data entry.

### Network Analysis & Visualization

- **Interactive World Map & 3D Globe**: A powerful visualization tool to plot flight paths, identify key operational hubs, and map relationships between entities. Click any airport to highlight all connected routes and destinations.

- **Route & Airport Analytics**: A dedicated analytics dashboard to identify the most flown routes and most frequently visited airports, complete with detailed statistics on flight counts, distances, and durations.

### Entity Resolution & Profiling

- **Passenger Management Interface**: A dedicated hub to manage all passenger entities extracted from flight logs.

- **Entity Resolution**: A simple, powerful interface to map cryptic abbreviations and aliases (e.g., "JE," "YY") to real-world identities.

- **Automated Dossier Generation**: Instantly generate comprehensive, data-rich profile cards for any entity, showing their complete travel history, top routes, and a full list of known travel companions and their shared flight counts.

### Automated OSINT Engine

- **Multi-Agent Workforce**: Deploy a suite of specialized AI research agents (Gemini, Grok, DeepSeek) to perform open-source intelligence gathering.

- **Universal Prompt System**: A template-based query engine that allows you to construct high-quality, data-enriched prompts using entities directly from your database (flights, passengers, locations, dates).

- **AI-Powered Cross-Referencing**: Task agents to find corroborating evidence for specific events by searching public news archives, event listings, and other open-source information, providing a powerful tool for verification.

- **Research Document Management**: All AI-generated reports are saved, categorized, and exportable to Markdown, creating a complete, auditable trail of your investigation.

## Tech Stack

### Backend
- **Rust** - High-performance, memory-safe systems language
- **Tauri 2.0** - Secure desktop application framework
- **SQLite** - Embedded database with rusqlite
- **Reqwest** - Async HTTP client for API calls
- **Serde** - Serialization/deserialization

### Frontend
- **Svelte 5** - Reactive UI framework with runes
- **SvelteKit** - Application framework
- **TypeScript** - Type-safe JavaScript
- **TailwindCSS 3.4** - Utility-first CSS framework

### AI Integrations
- **Google Gemini API** - Vision AI for OCR, chat capabilities
- **X.AI Grok API** - Real-time web search and analysis
- **DeepSeek API** - Advanced research and analysis

## Getting Started

### Prerequisites
- **Node.js** 18+ and npm
- **Rust** 1.70+
- **API Keys** (at least one):
  - Gemini: [Google AI Studio](https://makersuite.google.com/app/apikey)
  - Grok: [X.AI Console](https://console.x.ai)
  - DeepSeek: [DeepSeek Platform](https://platform.deepseek.com)

### Installation

1. **Clone the repository**:
```bash
git clone https://github.com/quantum-encoding/flight-tracker-pro-public.git
cd flight-tracker-pro-public
```

2. **Install dependencies**:
```bash
npm install
```

3. **Set up API keys** (choose one or more methods):

   **Option A: Environment Variables** (Recommended)
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   export GENAI_API_KEY="your-gemini-key"
   export XAI_API_KEY="your-grok-key"
   export DEEPSEEK_API_KEY="your-deepseek-key"
   ```

   **Option B: In-App Settings**
   - Launch the application
   - Open Settings tab
   - Enter API keys in the respective fields

4. **Run the application**:
```bash
# Development mode (hot reload)
npm run tauri dev

# Production build
npm run tauri build
```

### Quick Start Scripts

```bash
./dev.sh        # Development mode
./build.sh      # Production build
./clean.sh      # Clean build artifacts
```

## Application Structure

### Main Tabs

1. **Flights** - Flight list and management
2. **Map** - 2D and 3D geographic visualization
3. **Analytics** - Statistics and charts
4. **Passengers** - Passenger database and management
5. **Researchers** - AI research interface
6. **Reports** - Research documents and exports
7. **Settings** - Application configuration

### Data Storage

- **Database Location**: `~/.local/share/flight-tracker-pro/flight_tracker.db`
- **Schema**: SQLite with comprehensive tables for flights, users, passengers, and research reports

## Configuration

### API Keys

API keys can be set via:
1. Environment variables (persistent across app restarts)
2. Settings page (stored in database)

**Environment variables take precedence over database settings.**

### Supported Formats

**CSV Import**:
- Headers: Any format, smart detection enabled
- Required columns: Date, From (Origin), To (Destination)
- Optional columns: Passengers, Flight Number, Aircraft Registration
- Date format: Flexible (ISO 8601 recommended: YYYY-MM-DD)

**Airport Codes**:
- ICAO (4-letter): KJFK, EGLL, LFPG
- IATA (3-letter): JFK, LHR, CDG
- Both formats supported throughout the app

## Development

### Project Structure

```
flight-tracker-pro/
├── src/                    # Frontend (Svelte)
│   ├── lib/
│   │   └── components/     # Svelte components
│   └── routes/             # SvelteKit routes
├── src-tauri/              # Backend (Rust)
│   └── src/
│       ├── main.rs         # Application entry
│       ├── database.rs     # SQLite operations
│       ├── models.rs       # Data structures
│       ├── gemini.rs       # Gemini AI integration
│       ├── grok.rs         # Grok AI integration
│       ├── deepseek.rs     # DeepSeek AI integration
│       ├── ocr.rs          # OCR functionality
│       ├── geo.rs          # Geographic calculations
│       └── calculations.rs # Distance, fuel, etc.
├── README.md               # This file
└── package.json            # Node dependencies
```

### Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies
npm install

# Build frontend
npm run build

# Build Tauri app
npm run tauri build
```

### Development Mode

```bash
# Combined frontend + Tauri (recommended)
npm run tauri dev

# Or use the dev script
./dev.sh
```
## Roadmap

### Completed Features ✅
- [x] Flight database with CRUD operations
- [x] CSV import with preview and editing
- [x] Multi-agent AI research system
- [x] Individual agent chat interfaces
- [x] Template-based universal prompts
- [x] Research report management
- [x] Passenger tracking and mapping
- [x] Analytics and statistics
- [x] OCR with Gemini Vision
- [x] Markdown export for reports
- [x] Distance calculations
- [x] Multi-user support
- [x] Interactive map visualization with flight paths
- [x] Tax deduction calculations for business travel
- [x] Carbon footprint tracking
- [x] Frequent flyer program integration
- [x] External API integrations (FlightAware, etc.)
- [x] Photo attachments for trips
- [x] Journey/trip grouping
- [x] Pilot logbook mode (FAA/EASA compliant)
- [x] Data visualization improvements
- [ ] Mobile app (Tauri mobile support)

## Troubleshooting

### API Keys Not Working

**Issue**: "API key not configured" error

**Solutions**:
1. Check environment variables: `echo $GENAI_API_KEY`
2. Restart the app after setting environment variables
3. Try setting via Settings page instead
4. Verify key is valid on respective AI platform

### CSV Import Fails

**Issue**: Import button disabled or errors during import

**Solutions**:
1. Ensure CSV has headers in first row
2. Check date format (YYYY-MM-DD recommended)
3. Verify airport codes are valid ICAO/IATA
4. Use column mapping to manually assign fields
5. Check preview for validation errors (red highlights)

### Distance Shows as 0

**Issue**: Flights imported but distance is 0

**Solutions**:
1. Verify airport codes are correct
2. Both departure and arrival must be valid
3. Check if airports exist in airport database
4. Edit flight and re-save to recalculate

## Contributing

Contributions, suggestions, and feedback are welcome. Please open an issue for bugs or feature requests.

## License

MIT License - See LICENSE file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/) - Secure desktop app framework
- Powered by [Svelte 5](https://svelte.dev/) - Reactive UI framework
- AI capabilities from [Google Gemini](https://ai.google.dev/), [X.AI Grok](https://x.ai/), and [DeepSeek](https://www.deepseek.com/)
- Airport data from [OurAirports](https://ourairports.com/)
- Icons and UI components from TailwindCSS ecosystem

---

**Version**: 1.0.0
**Last Updated**: 2025-11-30
**Platforms**: Linux, Windows, macOS
