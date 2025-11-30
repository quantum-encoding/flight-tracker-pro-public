<script lang="ts">
  import '../app.css';
  import { initI18n } from '$lib/i18n';
  import { initTheme } from '$lib/theme';

  // Initialize i18n system on app start
  initI18n();

  // Initialize theme system on app start
  initTheme();
</script>

<div class="min-h-screen w-full bg-gray-50 dark:bg-gray-900 overflow-x-hidden">
  <slot />
</div>
