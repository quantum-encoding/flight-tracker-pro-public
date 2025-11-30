<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    userId: string;
    onClose: () => void;
    onSuccess: () => void;
  }

  let { userId, onClose, onSuccess }: Props = $props();

  // Form state
  let flightNumber = $state('');
  let departureAirport = $state('');
  let arrivalAirport = $state('');
  let departureDate = $state('');
  let departureTime = $state('');
  let arrivalDate = $state('');
  let arrivalTime = $state('');
  let aircraftRegistration = $state('');
  let aircraftType = $state('');
  let seatNumber = $state('');
  let bookingReference = $state('');
  let ticketNumber = $state('');
  let fareClass = $state('');
  let baseFare = $state('');
  let taxes = $state('');
  let totalCost = $state('');
  let currency = $state('USD');
  let notes = $state('');
  let passengerNames = $state('');

  let saving = $state(false);
  let error = $state<string | null>(null);
  let estimatedDistance = $state<number | null>(null);

  // Auto-calculate distance when airports change
  $effect(() => {
    if (departureAirport.length >= 3 && arrivalAirport.length >= 3) {
      calculateDistance();
    }
  });

  async function calculateDistance() {
    try {
      // This would call a backend function to calculate distance
      // For now, we'll let the backend handle it when creating the flight
      estimatedDistance = null;
    } catch (err) {
      console.error('Failed to calculate distance:', err);
    }
  }

  async function saveFlight() {
    // Validation
    if (!departureAirport.trim() || !arrivalAirport.trim()) {
      error = 'Departure and arrival airports are required';
      return;
    }

    if (!departureDate) {
      error = 'Departure date is required';
      return;
    }

    saving = true;
    error = null;

    try {
      // Construct datetime strings
      const departureDateTime = departureDate + (departureTime ? `T${departureTime}:00` : 'T00:00:00');
      const arrivalDateTime = arrivalDate && arrivalTime ? `${arrivalDate}T${arrivalTime}:00` : null;

      // Build notes with passenger names
      const finalNotes = passengerNames.trim()
        ? `Passengers: ${passengerNames.trim()}${notes.trim() ? '\n\n' + notes.trim() : ''}`
        : notes.trim() || null;

      // Parse numeric fields
      const baseFareNum = baseFare.trim() ? parseFloat(baseFare) : null;
      const taxesNum = taxes.trim() ? parseFloat(taxes) : null;
      const totalCostNum = totalCost.trim() ? parseFloat(totalCost) : null;

      const flightData = {
        id: '',
        flight_number: flightNumber.trim() || null,
        departure_airport: departureAirport.trim().toUpperCase(),
        arrival_airport: arrivalAirport.trim().toUpperCase(),
        departure_datetime: departureDateTime,
        arrival_datetime: arrivalDateTime,
        aircraft_registration: aircraftRegistration.trim() || null,
        aircraft_type_id: aircraftType.trim() || null,
        seat_number: seatNumber.trim() || null,
        booking_reference: bookingReference.trim() || null,
        ticket_number: ticketNumber.trim() || null,
        fare_class: fareClass.trim() || null,
        base_fare: baseFareNum,
        taxes: taxesNum,
        total_cost: totalCostNum,
        currency: currency.trim() || 'USD',
        notes: finalNotes,
        distance_nm: null, // Will be calculated by backend
        flight_duration: null, // Will be calculated by backend
        total_duration: null,
        attachment_path: null,
        created_at: '',
        updated_at: ''
      };

      await invoke('create_flight', { userId, flight: flightData });

      onSuccess();
      onClose();
    } catch (err) {
      console.error('Failed to save flight:', err);
      error = `Failed to save flight: ${err}`;
    } finally {
      saving = false;
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<!-- Modal Backdrop -->
<div
  class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
  onclick={handleBackdropClick}
  role="button"
  tabindex="0"
>
  <!-- Modal Container -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
    <!-- Header -->
    <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
          ✈️ Add Manual Flight
        </h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          Enter flight details manually
        </p>
      </div>
      <button
        onclick={onClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
        title="Close"
      >
        ×
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      {#if error}
        <div class="mb-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
        </div>
      {/if}

      <form onsubmit={(e) => { e.preventDefault(); saveFlight(); }} class="space-y-6">
        <!-- Flight Information -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Flight Information</h3>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label for="flight-number-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Flight Number <span class="text-gray-400">(Optional)</span>
              </label>
              <input
                id="flight-number-input"
                type="text"
                bind:value={flightNumber}
                placeholder="e.g., AA1234, UA567"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="aircraft-type-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Aircraft Type <span class="text-gray-400">(Optional)</span>
              </label>
              <input
                id="aircraft-type-input"
                type="text"
                bind:value={aircraftType}
                placeholder="e.g., Boeing 737, A320"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </section>

        <!-- Route Information -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Route Information</h3>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label for="departure-airport-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Departure Airport <span class="text-red-500">*</span>
              </label>
              <input
                id="departure-airport-input"
                type="text"
                bind:value={departureAirport}
                placeholder="IATA code (e.g., JFK, LAX)"
                required
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 uppercase"
                maxlength="4"
              />
            </div>

            <div>
              <label for="arrival-airport-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Arrival Airport <span class="text-red-500">*</span>
              </label>
              <input
                id="arrival-airport-input"
                type="text"
                bind:value={arrivalAirport}
                placeholder="IATA code (e.g., JFK, LAX)"
                required
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 uppercase"
                maxlength="4"
              />
            </div>
          </div>
        </section>

        <!-- Date & Time -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Date & Time</h3>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label for="departure-date-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Departure Date <span class="text-red-500">*</span>
              </label>
              <input
                id="departure-date-input"
                type="date"
                bind:value={departureDate}
                required
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="departure-time-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Departure Time <span class="text-gray-400">(Optional)</span>
              </label>
              <input
                id="departure-time-input"
                type="time"
                bind:value={departureTime}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="arrival-date-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Arrival Date <span class="text-gray-400">(Optional)</span>
              </label>
              <input
                id="arrival-date-input"
                type="date"
                bind:value={arrivalDate}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="arrival-time-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Arrival Time <span class="text-gray-400">(Optional)</span>
              </label>
              <input
                id="arrival-time-input"
                type="time"
                bind:value={arrivalTime}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </section>

        <!-- Booking Details -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Booking Details</h3>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label for="seat-number-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Seat Number
              </label>
              <input
                id="seat-number-input"
                type="text"
                bind:value={seatNumber}
                placeholder="e.g., 12A, 23F"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="booking-reference-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Booking Reference
              </label>
              <input
                id="booking-reference-input"
                type="text"
                bind:value={bookingReference}
                placeholder="e.g., ABC123"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 uppercase"
              />
            </div>

            <div>
              <label for="ticket-number-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Ticket Number
              </label>
              <input
                id="ticket-number-input"
                type="text"
                bind:value={ticketNumber}
                placeholder="e.g., 1234567890123"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="fare-class-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Fare Class
              </label>
              <select
                id="fare-class-input"
                bind:value={fareClass}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              >
                <option value="">Select...</option>
                <option value="Economy">Economy</option>
                <option value="Premium Economy">Premium Economy</option>
                <option value="Business">Business</option>
                <option value="First">First Class</option>
              </select>
            </div>

            <div>
              <label for="aircraft-registration-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Aircraft Registration
              </label>
              <input
                id="aircraft-registration-input"
                type="text"
                bind:value={aircraftRegistration}
                placeholder="e.g., N12345"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 uppercase"
              />
            </div>
          </div>
        </section>

        <!-- Cost Information -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Cost Information</h3>
          <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div>
              <label for="base-fare-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Base Fare
              </label>
              <input
                id="base-fare-input"
                type="number"
                step="0.01"
                bind:value={baseFare}
                placeholder="0.00"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="taxes-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Taxes & Fees
              </label>
              <input
                id="taxes-input"
                type="number"
                step="0.01"
                bind:value={taxes}
                placeholder="0.00"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="total-cost-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Total Cost
              </label>
              <input
                id="total-cost-input"
                type="number"
                step="0.01"
                bind:value={totalCost}
                placeholder="0.00"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label for="currency-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Currency
              </label>
              <select
                id="currency-input"
                bind:value={currency}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              >
                <option value="USD">USD</option>
                <option value="EUR">EUR</option>
                <option value="GBP">GBP</option>
                <option value="CAD">CAD</option>
                <option value="AUD">AUD</option>
                <option value="JPY">JPY</option>
              </select>
            </div>
          </div>
        </section>

        <!-- Passengers & Notes -->
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Additional Information</h3>
          <div class="space-y-4">
            <div>
              <label for="passenger-names-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Passenger Names <span class="text-gray-400">(comma-separated)</span>
              </label>
              <input
                id="passenger-names-input"
                type="text"
                bind:value={passengerNames}
                placeholder="e.g., John Smith, Jane Doe"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              />
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Enter full names or abbreviations separated by commas
              </p>
            </div>

            <div>
              <label for="notes-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Notes
              </label>
              <textarea
                id="notes-input"
                bind:value={notes}
                rows="3"
                placeholder="Add any additional notes about this flight..."
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
              ></textarea>
            </div>
          </div>
        </section>
      </form>
    </div>

    <!-- Footer -->
    <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900 flex items-center justify-between">
      <p class="text-sm text-gray-600 dark:text-gray-400">
        <span class="text-red-500">*</span> Required fields
      </p>
      <div class="flex gap-3">
        <button
          onclick={onClose}
          type="button"
          class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
        >
          Cancel
        </button>
        <button
          onclick={saveFlight}
          disabled={saving}
          type="submit"
          class="px-6 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition"
        >
          {saving ? 'Saving...' : '✓ Save Flight'}
        </button>
      </div>
    </div>
  </div>
</div>
