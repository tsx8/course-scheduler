<script setup>
import { onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { NConfigProvider, NMessageProvider, NDialogProvider, NNotificationProvider } from 'naive-ui';
import { useAuthStore } from './stores/auth';

const router = useRouter();
const authStore = useAuthStore();

const themeOverrides = {
  common: {
    // primaryColor: '#2080f0',
    // primaryColorHover: '#4098fc',
  }
};

// Redirect to login if not authenticated
onMounted(() => {
  if (!authStore.isAuthenticated && router.currentRoute.value.meta.requiresAuth !== false) {
    router.push('/login');
  }
});
</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <router-view />
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 1.5;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body,
html,
#app {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }
}
</style>