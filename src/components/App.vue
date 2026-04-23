<template>
  <div
    class="app-shell min-w-[1080px]"
    :style="{ height: clientHeight + 'px' }"
  >
    <NavBar />

    <div
      v-show="store.navView === 'solver'"
      class="app-main-height flex w-full mx-auto max-w-screen-xl"
    >
      <SideBar class="app-pane-height" />

      <div
        class="app-pane-height flex-grow my-4 px-6 pt-2 overflow-y-auto"
      >
        <div class="flex">
          <div
            class="theme-section-title mb-5 pl-2 pr-3 pb-0.5 text-lg font-bold border-l-8 border-b-2 rounded rounded-br-none"
          >
            {{ header }}
          </div>
        </div>

        <div v-if="store.sideView === 'about'">
          <AboutPage />
        </div>
        <div
          v-for="solverView in solverViews"
          :key="solverView.id"
          v-show="store.sideView === solverView.id"
        >
          <component :is="solverView.component" v-bind="solverView.props ?? {}" />
        </div>
      </div>
    </div>

    <div
      v-show="store.navView === 'results'"
      class="app-main-height overflow-y-auto"
    >
      <ResultViewer class="app-results-height" />
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent, ref } from "vue";
import { useStore } from "../store";

import NavBar from "./NavBar.vue";
import SideBar from "./SideBar.vue";
import AboutPage from "./AboutPage.vue";
import RangeEditor from "./RangeEditor.vue";
import BoardSelector from "./BoardSelector.vue";
import TreeConfig from "./TreeConfig.vue";
import RunSolver from "./RunSolver.vue";
import ResultViewer from "./ResultViewer.vue";

export default defineComponent({
  components: {
    NavBar,
    SideBar,
    AboutPage,
    RangeEditor,
    BoardSelector,
    TreeConfig,
    RunSolver,
    ResultViewer,
  },

  setup() {
    const store = useStore();
    store.initializeThemeMode();

    const header = computed(() => store.headers[store.sideView].join(" > "));
    const solverViews = [
      { id: "oop-range", component: RangeEditor, props: { player: 0 } },
      { id: "ip-range", component: RangeEditor, props: { player: 1 } },
      { id: "board", component: BoardSelector },
      { id: "tree-config", component: TreeConfig },
      { id: "run-solver", component: RunSolver },
    ];

    const clientHeight = ref(0);

    const updateClientHeight = () => {
      clientHeight.value = Math.min(
        document.documentElement.clientHeight - 0.01,
        Math.max(document.documentElement.clientWidth, 1080) * 0.8
      );
    };

    updateClientHeight();
    window.addEventListener("resize", updateClientHeight);

    return {
      store,
      header,
      solverViews,
      clientHeight,
    };
  },
});
</script>
