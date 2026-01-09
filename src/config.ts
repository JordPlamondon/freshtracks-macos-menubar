// API Configuration
export const API_BASE_URL = import.meta.env.DEV
  ? 'http://localhost:8000/api'  // Laravel dev server
  : 'https://getfreshtracks.com/api'  // Production URL

export const WS_URL = import.meta.env.DEV
  ? 'ws://localhost:6001'
  : 'wss://getfreshtracks.com'

export const NUXT_URL = import.meta.env.DEV
  ? 'http://localhost:3000'
  : 'https://getfreshtracks.com'
