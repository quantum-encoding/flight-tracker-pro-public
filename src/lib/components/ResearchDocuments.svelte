<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface Props {
    user: any;
  }

  let { user }: Props = $props();

  let reports: any[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedReport: any = $state(null);
  let totalReports = $state(0);

  onMount(async () => {
    await loadReports();
  });

  async function loadReports() {
    if (!user) return;

    loading = true;
    error = null;

    try {
      const [fetchedReports, count] = await Promise.all([
        invoke('list_research_reports', { userId: user.id, limit: 100, offset: 0 }),
        invoke('count_research_reports', { userId: user.id }),
      ]);

      reports = fetchedReports as any[];
      totalReports = count as number;
    } catch (err) {
      console.error('Failed to load reports:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function deleteReport(reportId: string) {
    if (!confirm('Are you sure you want to delete this report?')) {
      return;
    }

    try {
      await invoke('delete_research_report', { reportId });
      await loadReports();
      if (selectedReport?.id === reportId) {
        selectedReport = null;
      }
    } catch (err) {
      console.error('Failed to delete report:', err);
      alert(`Failed to delete report: ${err}`);
    }
  }

  function viewReport(report: any) {
    selectedReport = report;
  }

  function closeReport() {
    selectedReport = null;
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getAgentIcon(agentName: string): string {
    const icons: Record<string, string> = {
      Gemini: '🔮',
      Grok: '🧠',
      DeepSeek: '🤖',
      Local: '💻',
    };
    return icons[agentName] || '📄';
  }

  function getAgentColor(agentName: string): string {
    const colors: Record<string, string> = {
      Gemini: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      Grok: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
      DeepSeek: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200',
      Local: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    };
    return colors[agentName] || 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200';
  }

  function parseSources(sourcesJson: string | null): any[] {
    if (!sourcesJson) return [];
    try {
      return JSON.parse(sourcesJson);
    } catch {
      return [];
    }
  }

  function parseReportDetails(detailsJson: string | null): any {
    if (!detailsJson) return null;
    try {
      return JSON.parse(detailsJson);
    } catch {
      return null;
    }
  }

  async function exportToMarkdown(reportId: string) {
    try {
      const filePath = await invoke('export_research_report_to_markdown', { reportId });
      alert(`Report exported successfully to:\n${filePath}`);
    } catch (err) {
      console.error('Failed to export report:', err);
      alert(`Failed to export report: ${err}`);
    }
  }
</script>

<div class="p-6 max-w-7xl mx-auto">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
      📚 Research Documents
    </h1>
    <p class="text-gray-600 dark:text-gray-400">
      Saved AI research reports and findings ({totalReports} total)
    </p>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-16">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
    </div>
  {:else if error}
    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6">
      <div class="flex items-start">
        <span class="text-4xl">⚠️</span>
        <div class="ml-4">
          <h3 class="text-lg font-semibold text-red-900 dark:text-red-200 mb-2">Failed to Load Reports</h3>
          <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
          <button
            onclick={loadReports}
            class="mt-4 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg font-medium transition"
          >
            Try Again
          </button>
        </div>
      </div>
    </div>
  {:else if reports.length === 0}
    <div class="text-center py-16">
      <div class="text-6xl mb-4">📭</div>
      <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">
        No Reports Yet
      </h3>
      <p class="text-gray-600 dark:text-gray-400">
        Use the Researchers tab to create your first AI research report
      </p>
    </div>
  {:else if !selectedReport}
    <!-- Reports List -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each reports as report}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md hover:shadow-lg transition-shadow overflow-hidden">
          <!-- Report Header -->
          <div class="p-4 border-b border-gray-200 dark:border-gray-700">
            <div class="flex items-start justify-between mb-2">
              <div class="flex items-center gap-2">
                <span class="text-2xl">{getAgentIcon(report.agent_name)}</span>
                <span class="px-2 py-1 rounded text-xs font-semibold {getAgentColor(report.agent_name)}">
                  {report.agent_name}
                </span>
              </div>
              <button
                onclick={() => deleteReport(report.id)}
                class="text-gray-400 hover:text-red-600 dark:hover:text-red-400 transition"
                title="Delete report"
              >
                🗑️
              </button>
            </div>
            {#if report.agent_model}
              <div class="text-xs text-gray-500 dark:text-gray-400">
                Model: {report.agent_model}
              </div>
            {/if}
          </div>

          <!-- Report Content -->
          <div class="p-4">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-2 line-clamp-2">
              {report.search_query}
            </h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-3 mb-3">
              {report.report_summary}
            </p>

            <!-- Metadata -->
            <div class="text-xs text-gray-500 dark:text-gray-500 space-y-1">
              <div>📅 {formatDate(report.created_at)}</div>
              {#if report.confidence_score}
                <div>🎯 Confidence: {Math.round(report.confidence_score * 100)}%</div>
              {/if}
              {#if report.processing_time_ms}
                <div>⏱️ {report.processing_time_ms}ms</div>
              {/if}
            </div>
          </div>

          <!-- View Button -->
          <div class="p-4 pt-0">
            <button
              onclick={() => viewReport(report)}
              class="w-full bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg font-medium transition text-sm"
            >
              View Full Report
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- Report Detail View -->
    <div class="max-w-4xl mx-auto">
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg overflow-hidden">
        <!-- Header -->
        <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between bg-gradient-to-r from-gray-50 to-white dark:from-gray-900 dark:to-gray-800">
          <div class="flex items-center gap-3">
            <span class="text-3xl">{getAgentIcon(selectedReport.agent_name)}</span>
            <div>
              <div class="flex items-center gap-2 mb-1">
                <span class="px-2 py-1 rounded text-xs font-semibold {getAgentColor(selectedReport.agent_name)}">
                  {selectedReport.agent_name}
                </span>
                {#if selectedReport.agent_model}
                  <span class="text-xs text-gray-500 dark:text-gray-400">
                    {selectedReport.agent_model}
                  </span>
                {/if}
              </div>
              <div class="text-xs text-gray-600 dark:text-gray-400">
                {formatDate(selectedReport.created_at)}
              </div>
            </div>
          </div>
          <button
            onclick={closeReport}
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold"
            title="Close"
          >
            ×
          </button>
        </div>

        <!-- Content -->
        <div class="p-6 space-y-6">
          <!-- Search Query -->
          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
            <h3 class="text-sm font-semibold text-blue-900 dark:text-blue-200 mb-2">🔍 Research Query</h3>
            <p class="text-blue-800 dark:text-blue-300">{selectedReport.search_query}</p>
          </div>

          <!-- Metadata -->
          <div class="flex flex-wrap gap-4 text-sm">
            {#if selectedReport.confidence_score}
              <div class="flex items-center gap-2">
                <span class="text-gray-500 dark:text-gray-400">Confidence:</span>
                <span class="font-semibold text-gray-900 dark:text-white">
                  {Math.round(selectedReport.confidence_score * 100)}%
                </span>
              </div>
            {/if}
            {#if selectedReport.processing_time_ms}
              <div class="flex items-center gap-2">
                <span class="text-gray-500 dark:text-gray-400">Processing Time:</span>
                <span class="font-semibold text-gray-900 dark:text-white">
                  {selectedReport.processing_time_ms}ms
                </span>
              </div>
            {/if}
            <div class="flex items-center gap-2">
              <span class="text-gray-500 dark:text-gray-400">Report Type:</span>
              <span class="font-semibold text-gray-900 dark:text-white capitalize">
                {selectedReport.report_type}
              </span>
            </div>
          </div>

          <!-- Summary -->
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">📊 Summary</h3>
            <p class="text-gray-700 dark:text-gray-300 leading-relaxed">
              {selectedReport.report_summary}
            </p>
          </div>

          <!-- Full Details (if available) -->
          {#if selectedReport.report_details}
            {@const details = parseReportDetails(selectedReport.report_details)}
            {#if details && details.key_findings && details.key_findings.length > 0}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">🔍 Key Findings</h3>
                <div class="space-y-3">
                  {#each details.key_findings as finding}
                    <div class="p-4 bg-gray-50 dark:bg-gray-900 rounded-lg border-l-4 border-primary-500">
                      <div class="flex items-start justify-between mb-1">
                        <span class="font-semibold text-gray-900 dark:text-white">{finding.category}</span>
                        {#if finding.relevance}
                          <span class="text-xs px-2 py-0.5 rounded bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                            {Math.round(finding.relevance * 100)}%
                          </span>
                        {/if}
                      </div>
                      <p class="text-sm text-gray-600 dark:text-gray-400">{finding.description}</p>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          {/if}

          <!-- Sources -->
          {#if selectedReport.sources}
            {@const sources = parseSources(selectedReport.sources)}
            {#if sources.length > 0}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">📚 Sources</h3>
                <div class="space-y-3">
                  {#each sources as source}
                    <div class="p-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
                      <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{source.title}</h4>
                      <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">{source.snippet}</p>
                      {#if source.url}
                        <a
                          href={source.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="text-xs text-primary-600 hover:text-primary-800 dark:text-primary-400 dark:hover:text-primary-300 hover:underline break-all"
                        >
                          🔗 {source.url}
                        </a>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          {/if}
        </div>

        <!-- Footer -->
        <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
          <div class="flex gap-3">
            <button
              onclick={closeReport}
              class="flex-1 bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              ← Back to List
            </button>
            <button
              onclick={() => exportToMarkdown(selectedReport.id)}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg font-medium transition"
              title="Export to Markdown"
            >
              📄 Export
            </button>
            <button
              onclick={() => deleteReport(selectedReport.id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition"
            >
              🗑️ Delete
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
