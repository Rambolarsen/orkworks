<script setup>
import { data } from "../site.data.mts";
</script>
<template>
  <div class="download-links">
    <template v-if="data.release">
      <p>
        Latest public release:
        <a :href="data.release.url">{{ data.release.version }}</a>
      </p>
      <div class="download-actions" v-if="data.release.downloads.length">
        <a
          v-for="download in data.release.downloads"
          :key="download.url"
          class="site-button secondary"
          :href="download.url"
          >Download for {{ download.label }} ↗</a
        >
      </div>
      <p v-else>
        See the release page for available files, or build from source below.
      </p>
    </template>
    <p v-else-if="data.checkedReleases">
      Public installers aren’t available yet. You can try OrkWorks from source
      today.
    </p>
    <p v-else>
      <a href="https://github.com/Rambolarsen/orkworks/releases"
        >Check public releases for installers</a
      >, or try OrkWorks from source.
    </p>
  </div>
</template>
