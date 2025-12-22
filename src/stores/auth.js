// ============================================================================
// Authentication Store
// Feature: 001-rbac-audit-system
// ============================================================================

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useDataStore } from './data'

export const useAuthStore = defineStore('auth', () => {
  // State
  const currentUser = ref(null)
  const sessionId = ref(null)

  // Computed properties
  const isAuthenticated = computed(() => {
    return currentUser.value !== null && sessionId.value !== null
  })

  const isScheduler = computed(() => {
    return currentUser.value?.role === 'Scheduler' || currentUser.value?.role_id === '00000000-0000-0000-0000-000000000001'
  })

  const isTeacher = computed(() => {
    return currentUser.value?.role === 'Teacher' || currentUser.value?.role_id === '00000000-0000-0000-0000-000000000002'
  })

  // Actions
  async function login(username, password) {
    try {
      const result = await invoke('authenticate_user', { username, password })
      currentUser.value = result.user
      sessionId.value = result.session_id

      localStorage.setItem('session_id', result.session_id)

      return result
    } catch (error) {
      console.error('Login failed:', error)
      throw error
    }
  }

  async function logout() {
    const dataStore = useDataStore();
    try {
      if (sessionId.value) {
        await invoke('logout_user', { sessionId: sessionId.value })
      }
    } catch (error) {
      console.error('Logout error:', error)
    } finally {
      dataStore.resetState();
      // Clear session regardless of server response
      currentUser.value = null
      sessionId.value = null
      localStorage.removeItem('session_id')
    }
  }

  async function restoreSession() {
    const savedId = localStorage.getItem('session_id')
    if (!savedId) return false

    try {
      const user = await invoke('get_current_user', { sessionId: savedId })
      currentUser.value = user
      sessionId.value = savedId
      return true
    } catch (error) {
      console.error('Restore session failed:', error)
      localStorage.removeItem('session_id')
      return false
    }
  }

  return {
    // State
    currentUser,
    sessionId,

    // Computed
    isAuthenticated,
    isScheduler,
    isTeacher,

    // Actions
    login,
    logout,
    restoreSession
  }
})
