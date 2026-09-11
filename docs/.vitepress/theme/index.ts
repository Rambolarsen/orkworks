import DefaultTheme from "vitepress/theme";
import LandingPage from "./LandingPage.vue";
import DownloadLinks from "./DownloadLinks.vue";
import "./style.css";
export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("LandingPage", LandingPage);
    app.component("DownloadLinks", DownloadLinks);
  },
};
