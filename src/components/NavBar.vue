<template>
  <nav class="theme-nav sticky flex top-0 z-30 w-full h-10 px-4 justify-center">
    <div class="flex relative w-full">
      <div class="flex items-center z-10">
        <span class="px-4 text-lg font-semibold">WASM Postflop</span>
      </div>

      <div class="flex ml-auto h-full items-center z-10">
        <button
          class="theme-nav-item flex px-3 h-full items-center font-semibold"
          :aria-label="
            store.themeMode === 'dark'
              ? 'Switch to light theme'
              : 'Switch to dark theme'
          "
          :title="
            store.themeMode === 'dark'
              ? 'Switch to light theme'
              : 'Switch to dark theme'
          "
          @click="store.toggleThemeMode()"
        >
          <MoonIcon v-if="store.themeMode === 'light'" class="w-5 h-5 mr-2" />
          <SunIcon v-else class="w-5 h-5 mr-2" />
          {{ store.themeMode === "dark" ? "Dark" : "Light" }}
        </button>

        <a
          href="https://github.com/b-inary/wasm-postflop"
          class="theme-nav-item flex px-4 h-full items-center font-semibold"
          target="_blank"
        >
          <img
            src="../assets/GitHub-Mark-Light-32px.png"
            class="inline-block w-5 h-5 mr-2.5"
          />
          GitHub
        </a>
      </div>

      <div
        class="flex absolute w-full h-full left-0 top-0 gap-3 justify-center"
      >
        <button
          :class="
            'theme-nav-tab flex relative w-32 items-center justify-center font-semibold ' +
            (store.navView === 'solver' ? 'theme-nav-tab-active' : '')
          "
          @click="store.navView = 'solver'"
        >
          <ComputerDesktopIcon class="w-6 h-6" />
          <span class="pl-3">Solver</span>
        </button>
        <button
          :class="
            'theme-nav-tab flex relative w-32 items-center justify-center font-semibold ' +
            (store.navView === 'results' ? 'theme-nav-tab-active' : '')
          "
          @click="store.navView = 'results'"
        >
          <ChartBarIcon class="w-6 h-6" />
          <span class="pl-3">Results</span>
        </button>
      </div>
    </div>
  </nav>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useStore } from "../store";

import { ComputerDesktopIcon, ChartBarIcon } from "@heroicons/vue/24/solid";
import { MoonIcon, SunIcon } from "@heroicons/vue/24/outline";

export default defineComponent({
  components: {
    ComputerDesktopIcon,
    ChartBarIcon,
    MoonIcon,
    SunIcon,
  },
  setup() {
    return {
      store: useStore(),
    };
  },
});
</script>
