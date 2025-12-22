import { createApp } from "vue";
import { createPinia } from 'pinia';
import App from "./App.vue";
import router from './router';

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => e.preventDefault());
}

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(router);
app.mount("#app");
