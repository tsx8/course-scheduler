<template>
  <div class="login-container">
    <n-card class="login-card">
      <n-form ref="formRef" :model="formValue" :rules="rules" size="large">
        <n-form-item path="username" label="用户名">
          <n-input v-model:value="formValue.username" placeholder="请输入用户名" @keyup.enter="handleLogin" />
        </n-form-item>
        <n-form-item path="password" label="密码">
          <n-input v-model:value="formValue.password" type="password" show-password-on="click" placeholder="请输入密码"
            @keyup.enter="handleLogin" />
        </n-form-item>
        <n-form-item>
          <n-button type="primary" block :loading="loading" @click="handleLogin">
            登录
          </n-button>
        </n-form-item>
      </n-form>
    </n-card>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()

const formRef = ref(null)
const loading = ref(false)
const errorMessage = ref('')

const formValue = ref({
  username: '',
  password: ''
})

const rules = {
  username: [
    {
      required: true,
      message: '请输入用户名',
      trigger: 'blur'
    }
  ],
  password: [
    {
      required: true,
      message: '请输入密码',
      trigger: 'blur'
    }
  ]
}

const handleLogin = async () => {
  errorMessage.value = ''

  try {
    await formRef.value?.validate()
  } catch {
    return
  }

  loading.value = true

  try {
    await authStore.login(formValue.value.username, formValue.value.password)

    message.success('登录成功')

    if (authStore.isScheduler) {
      router.push('/campus-timetable')
    } else if (authStore.isTeacher) {
      router.push('/teacher-timetable')
    } else {
      router.push('/')
    }
  } catch (error) {
    console.error('Login error:', error)
    message.error('用户名或密码错误')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: calc(100vh - 48px);
  background: linear-gradient(135deg, #79F1A4 0%, #0e5cad 100%);
}

.login-card {
  width: 100%;
  max-width: 400px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
}
</style>
