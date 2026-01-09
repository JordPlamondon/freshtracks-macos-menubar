<template>
  <div class="menubar-popup" @click="hideContextMenu">
    <!-- Loading State -->
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
      <div class="loading-text">Loading...</div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-state">
      <div class="error-icon">⚠️</div>
      <div class="error-text">{{ error }}</div>
      <button @click="refresh" class="retry-button">Retry</button>
    </div>

    <!-- Main Content -->
    <template v-else>
      <!-- Date Header -->
      <div class="date-header">
        <div class="date-header-left">
          <button class="nav-arrow" @click="goToPreviousDay" title="Previous day">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M15 18l-6-6 6-6" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button class="nav-arrow" @click="goToNextDay" title="Next day">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>
        <span class="date-text">{{ selectedDateFormatted }}</span>
        <button class="calendar-icon" @click="goToToday" title="Go to today">
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="3" y="4" width="18" height="18" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M3 9H21" stroke="currentColor" stroke-width="1.5"/>
            <path d="M8 2V5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <path d="M16 2V5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span class="calendar-day">{{ todayDayNumber }}</span>
        </button>
      </div>

      <!-- Week Day Bar -->
      <div class="week-bar">
        <button
          v-for="day in weekDays"
          :key="day.date"
          @click="selectDay(day.date)"
          class="day-button"
          :class="{ 'active': day.date === selectedDate, 'today': day.isToday }"
        >
          <span class="day-letter">{{ day.letter }}</span>
          <span class="day-total">{{ formatDayTotal(day.total) }}</span>
        </button>
      </div>

      <!-- Day Header (hidden when any form is open) -->
      <div v-if="!showNewEntryForm && !showEditEntryForm" class="day-header">
        <span class="day-title">{{ selectedDayTitle }}</span>
        <span v-if="showLiveRevenue" class="day-revenue">
          <RollingNumber :value="selectedDayRevenue" />
        </span>
        <span v-else class="day-duration">{{ selectedDayTotal }}</span>
      </div>

      <!-- New Entry Form Header -->
      <div v-if="showNewEntryForm" class="day-header form-header">
        <span class="day-title">New Time Entry</span>
        <button @click="closeNewEntryForm" class="close-form-btn">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 18L18 6M6 6l12 12" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- Edit Entry Form Header -->
      <div v-if="showEditEntryForm" class="day-header form-header">
        <span class="day-title">Edit Time Entry</span>
        <button @click="closeEditEntryForm" class="close-form-btn">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 18L18 6M6 6l12 12" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- Entries List (hidden when any form is open) -->
      <div v-if="!showNewEntryForm && !showEditEntryForm" class="entries-list">
        <div v-if="selectedDayEntries.length === 0" class="no-entries">
          No entries for this day
        </div>
        <div
          v-for="entry in selectedDayEntries"
          :key="entry.id"
          class="entry-item"
          :class="{ 'running': !entry.stopped_at }"
          @contextmenu="showContextMenu($event, entry)"
        >
          <div class="entry-info">
            <div class="entry-client">{{ entry.project?.client?.name || 'No Client' }}</div>
            <div class="entry-project">{{ entry.project?.name || 'No Project' }}</div>
            <div v-if="entry.description" class="entry-description">{{ entry.description }}</div>
          </div>
          <div class="entry-actions">
            <span class="entry-duration" :class="{ 'running': !entry.stopped_at }">
              {{ entry.stopped_at ? formatDuration(entry.duration_minutes) : getLiveDuration(entry) }}
            </span>
            <!-- Stop button for running entry -->
            <button
              v-if="!entry.stopped_at"
              @click="handleStopEntry(entry)"
              class="action-btn stop-btn"
              :disabled="stoppingId === entry.id"
            >
              <span v-if="stoppingId === entry.id" class="spinner"></span>
              <span v-else>■</span>
            </button>
            <!-- Play button for stopped entry -->
            <button
              v-else
              @click="handleRestartEntry(entry)"
              class="action-btn play-btn"
              :disabled="restartingId === entry.id"
            >
              <span v-if="restartingId === entry.id" class="spinner"></span>
              <span v-else>▶</span>
            </button>
          </div>
        </div>
      </div>

      <!-- New Entry Form (inline, replaces entries list) -->
      <div v-if="showNewEntryForm" class="new-entry-form">
        <!-- Client Select -->
        <div class="form-group">
          <label class="form-label">Client</label>
          <select v-model="newEntryClientId" @change="onClientChange" class="form-select">
            <option value="">Select a client...</option>
            <option v-for="client in clients" :key="client.id" :value="client.id">
              {{ client.name }}
            </option>
          </select>
        </div>

        <!-- Project Select -->
        <div class="form-group">
          <label class="form-label">Project</label>
          <select v-model="newEntryProjectId" class="form-select" :disabled="!newEntryClientId || loadingProjects">
            <option value="">{{ loadingProjects ? 'Loading...' : 'Select a project...' }}</option>
            <option v-for="project in clientProjects" :key="project.id" :value="project.id">
              {{ project.name }}
            </option>
          </select>
        </div>

        <!-- Description -->
        <div class="form-group">
          <label class="form-label">Description (optional)</label>
          <input
            v-model="newEntryDescription"
            type="text"
            class="form-input"
            placeholder="What are you working on?"
          />
        </div>
      </div>

      <!-- New Entry Form Button (fixed at bottom) -->
      <div v-if="showNewEntryForm" class="form-button-container">
        <button
          @click="handleStartNewEntry"
          class="start-timer-btn"
          :disabled="!newEntryProjectId || startingNewEntry"
        >
          {{ startingNewEntry ? 'Starting...' : 'Start Timer' }}
        </button>
      </div>

      <!-- Edit Entry Form (inline, replaces entries list) -->
      <div v-if="showEditEntryForm" class="edit-entry-form">
        <!-- Client Select -->
        <div class="form-group">
          <label class="form-label">Client</label>
          <select v-model="editEntryClientId" @change="onEditClientChange" class="form-select">
            <option value="">Select a client...</option>
            <option v-for="client in clients" :key="client.id" :value="client.id">
              {{ client.name }}
            </option>
          </select>
        </div>

        <!-- Project Select -->
        <div class="form-group">
          <label class="form-label">Project</label>
          <select v-model="editEntryProjectId" class="form-select" :disabled="!editEntryClientId || loadingEditProjects">
            <option value="">{{ loadingEditProjects ? 'Loading...' : 'Select a project...' }}</option>
            <option v-for="project in editClientProjects" :key="project.id" :value="project.id">
              {{ project.name }}
            </option>
          </select>
        </div>

        <!-- Description -->
        <div class="form-group">
          <label class="form-label">Description</label>
          <textarea
            v-model="editEntryDescription"
            class="form-textarea"
            rows="2"
            placeholder="What were you working on?"
          ></textarea>
        </div>

        <!-- Date -->
        <div class="form-group">
          <label class="form-label">Date</label>
          <input
            v-model="editEntryDate"
            type="date"
            class="form-input"
          />
        </div>

        <!-- Time Range -->
        <div class="form-row">
          <div class="form-group form-group-half">
            <label class="form-label">Start Time</label>
            <input
              v-model="editEntryStartTime"
              type="time"
              step="1"
              class="form-input"
            />
          </div>
          <div class="form-group form-group-half">
            <label class="form-label">End Time</label>
            <input
              v-model="editEntryEndTime"
              type="time"
              step="1"
              class="form-input"
              :disabled="!editingEntry?.stopped_at"
              :placeholder="!editingEntry?.stopped_at ? 'Running...' : ''"
            />
          </div>
        </div>

        <!-- Billable Toggle -->
        <div class="form-group-inline">
          <input
            type="checkbox"
            id="edit-billable"
            v-model="editEntryBillable"
            class="form-checkbox"
          />
          <label for="edit-billable" class="form-label-inline">Billable</label>
        </div>
      </div>

      <!-- Edit Entry Form Button (fixed at bottom) -->
      <div v-if="showEditEntryForm" class="form-button-container">
        <button
          @click="handleSaveEntry"
          class="save-entry-btn"
          :disabled="!editEntryProjectId || savingEntry"
        >
          {{ savingEntry ? 'Saving...' : 'Save Changes' }}
        </button>
      </div>

      <!-- Footer (hide + button when form is open) -->
      <div class="footer">
        <button @click="openFullApp" class="footer-button">
          Open FreshTracks
        </button>
        <button v-if="!showNewEntryForm && !showEditEntryForm" @click="openNewEntryForm" class="footer-button-add" title="New Entry">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 5v14M5 12h14" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

    </template>

    <!-- Context Menu (teleported to body to avoid overflow clipping) -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        @click.stop
      >
        <button @click="handleContextEdit" class="context-menu-item">
          <svg class="context-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
          </svg>
          Edit Entry
        </button>
        <button
          @click="contextMenu.entry?.stopped_at ? handleContextStartTimer() : handleContextStopTimer()"
          class="context-menu-item"
        >
          <svg v-if="contextMenu.entry?.stopped_at" class="context-icon" fill="currentColor" viewBox="0 0 24 24">
            <path d="M8 5v14l11-7z"/>
          </svg>
          <svg v-else class="context-icon" fill="currentColor" viewBox="0 0 24 24">
            <path d="M6 6h12v12H6z"/>
          </svg>
          {{ contextMenu.entry?.stopped_at ? 'Start Timer' : 'Stop Timer' }}
        </button>
        <button @click="handleContextDelete" class="context-menu-item context-menu-item-danger">
          <svg class="context-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          Delete Entry
        </button>
        <div class="context-menu-separator"></div>
        <button @click="handleContextViewInApp" class="context-menu-item">
          <svg class="context-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
          </svg>
          View in FreshTracks
        </button>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { api } from './services/api'
import { open } from '@tauri-apps/plugin-shell'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { NUXT_URL } from './config'
import type { TimeEntry, Client, Project, UserSettings } from './services/api'
import RollingNumber from './components/RollingNumber.vue'

const entries = ref<TimeEntry[]>([])
const loading = ref(true)
const error = ref<string | null>(null)
const selectedDate = ref('')
const stoppingId = ref<number | null>(null)
const restartingId = ref<number | null>(null)
const currentTime = ref(Date.now())
const deletingId = ref<number | null>(null)
const userSettings = ref<UserSettings>({})

const showNewEntryForm = ref(false)
const clients = ref<Client[]>([])
const clientProjects = ref<Project[]>([])
const newEntryClientId = ref<number | ''>('')
const newEntryProjectId = ref<number | ''>('')
const newEntryDescription = ref('')
const loadingProjects = ref(false)
const startingNewEntry = ref(false)

const showEditEntryForm = ref(false)
const editingEntry = ref<TimeEntry | null>(null)
const editEntryClientId = ref<number | ''>('')
const editEntryProjectId = ref<number | ''>('')
const editClientProjects = ref<Project[]>([])
const loadingEditProjects = ref(false)
const editEntryDescription = ref('')
const editEntryDate = ref('')
const editEntryStartTime = ref('')
const editEntryEndTime = ref('')
const editEntryBillable = ref(true)
const savingEntry = ref(false)

const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  entry: null as TimeEntry | null
})

let timerInterval: number | null = null
let unlistenWsTimerStarted: (() => void) | null = null
let unlistenWsTimerStopped: (() => void) | null = null
let unlistenWsTimerDeleted: (() => void) | null = null
let unlistenWindowFocus: (() => void) | null = null

function getTodayStr(): string {
  const today = new Date()
  return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`
}

selectedDate.value = getTodayStr()

const weekDays = computed(() => {
  const todayStr = getTodayStr()

  // Parse selected date to get its week
  const [year, month, day] = selectedDate.value.split('-').map(Number)
  const selected = new Date(year, month - 1, day)
  const dayOfWeek = selected.getDay() // 0 = Sunday
  const monday = new Date(selected)
  monday.setDate(selected.getDate() - (dayOfWeek === 0 ? 6 : dayOfWeek - 1))

  const days = []
  const letters = ['M', 'T', 'W', 'T', 'F', 'S', 'S']

  for (let i = 0; i < 7; i++) {
    const date = new Date(monday)
    date.setDate(monday.getDate() + i)
    const dateStr = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`

    // Calculate total for this day
    const dayEntries = entries.value.filter(e => {
      const entryDate = new Date(e.started_at)
      const entryDateStr = `${entryDate.getFullYear()}-${String(entryDate.getMonth() + 1).padStart(2, '0')}-${String(entryDate.getDate()).padStart(2, '0')}`
      return entryDateStr === dateStr
    })

    let total = 0
    dayEntries.forEach(e => {
      if (e.stopped_at) {
        total += e.duration_minutes || 0
      } else {
        // Running timer - calculate live
        const sessionStart = new Date(e.resumed_at || e.started_at)
        const currentSessionMs = Math.max(0, currentTime.value - sessionStart.getTime())
        const currentSessionMins = currentSessionMs / 60000
        total += (e.duration_minutes || 0) + currentSessionMins
      }
    })

    days.push({
      date: dateStr,
      letter: letters[i],
      isToday: dateStr === todayStr,
      total
    })
  }

  return days
})

const selectedDayEntries = computed(() => {
  return entries.value.filter(e => {
    const entryDate = new Date(e.started_at)
    const entryDateStr = `${entryDate.getFullYear()}-${String(entryDate.getMonth() + 1).padStart(2, '0')}-${String(entryDate.getDate()).padStart(2, '0')}`
    return entryDateStr === selectedDate.value
  }).sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime())
})

const selectedDayTitle = computed(() => {
  const [year, month, day] = selectedDate.value.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  const todayStr = getTodayStr()

  if (selectedDate.value === todayStr) {
    return 'Today'
  }

  const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
  const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  return `${dayNames[date.getDay()]}, ${monthNames[date.getMonth()]} ${day}`
})

const selectedDateFormatted = computed(() => {
  const [year, month, day] = selectedDate.value.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  const dayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
  const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  return `${dayNames[date.getDay()]}, ${day} ${monthNames[date.getMonth()]}`
})

const todayDayNumber = computed(() => {
  return new Date().getDate()
})

const selectedDayTotal = computed(() => {
  let total = 0
  selectedDayEntries.value.forEach(e => {
    if (e.stopped_at) {
      total += e.duration_minutes || 0
    } else {
      const sessionStart = new Date(e.resumed_at || e.started_at)
      const currentSessionMs = Math.max(0, currentTime.value - sessionStart.getTime())
      const currentSessionMins = currentSessionMs / 60000
      total += (e.duration_minutes || 0) + currentSessionMins
    }
  })
  return formatDuration(total)
})

const selectedDayRevenue = computed(() => {
  let revenue = 0
  selectedDayEntries.value.forEach(e => {
    if (!e.is_billable) return
    const hourlyRate = e.project?.client?.hourly_rate || 0
    if (hourlyRate === 0) return

    if (e.stopped_at) {
      // Completed entry
      const hours = (e.duration_minutes || 0) / 60
      revenue += hours * hourlyRate
    } else {
      // Running timer - calculate live
      const sessionStart = new Date(e.resumed_at || e.started_at)
      const currentSessionMs = Math.max(0, currentTime.value - sessionStart.getTime())
      const currentSessionMins = currentSessionMs / 60000
      const totalMins = (e.duration_minutes || 0) + currentSessionMins
      const hours = totalMins / 60
      revenue += hours * hourlyRate
    }
  })
  return revenue
})

const showLiveRevenue = computed(() => {
  return userSettings.value.show_live_revenue === true
})

function selectDay(date: string) {
  selectedDate.value = date
}

function goToToday() {
  selectedDate.value = getTodayStr()
}

function goToPreviousDay() {
  const [year, month, day] = selectedDate.value.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  date.setDate(date.getDate() - 1)
  selectedDate.value = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function goToNextDay() {
  const [year, month, day] = selectedDate.value.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  date.setDate(date.getDate() + 1)
  selectedDate.value = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function formatDayTotal(minutes: number): string {
  const hours = Math.floor(minutes / 60)
  const mins = Math.floor(minutes % 60)
  return `${hours}:${String(mins).padStart(2, '0')}`
}

function formatDuration(minutes: number | null): string {
  if (!minutes) return '0:00'
  const totalMins = Math.floor(minutes)
  const hours = Math.floor(totalMins / 60)
  const mins = totalMins % 60
  return `${hours}:${String(mins).padStart(2, '0')}`
}

function getLiveDuration(entry: TimeEntry): string {
  const sessionStart = new Date(entry.resumed_at || entry.started_at)
  const currentSessionMs = Math.max(0, currentTime.value - sessionStart.getTime())
  const currentSessionSecs = Math.floor(currentSessionMs / 1000)
  const accumulatedSecs = Math.floor((entry.duration_minutes || 0) * 60)
  const totalSecs = accumulatedSecs + currentSessionSecs

  const hours = Math.floor(totalSecs / 3600)
  const mins = Math.floor((totalSecs % 3600) / 60)
  const secs = totalSecs % 60
  return `${hours}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
}

async function fetchEntries() {
  try {
    entries.value = await api.getTimeEntries()
  } catch (err) {
    console.error('Failed to fetch entries:', err)
  }
}

async function handleStopEntry(entry: TimeEntry) {
  stoppingId.value = entry.id
  try {
    const stopped = await api.stopTimer(entry.id)
    if (stopped) {
      const index = entries.value.findIndex(e => e.id === entry.id)
      if (index !== -1) {
        entries.value[index] = stopped
      }
      api.updateTrayStatus(false)
      api.clearTrayTimer() // Reset menu bar to just show icon
      api.showNotification('Timer Stopped', `${entry.project?.name || 'Timer'} - ${formatDuration(stopped.duration_minutes)}`)
    }
  } catch (err) {
    console.error('Failed to stop entry:', err)
  } finally {
    stoppingId.value = null
  }
}

async function handleRestartEntry(entry: TimeEntry) {
  restartingId.value = entry.id
  try {
    const restarted = await api.restartTimer(entry.id)
    if (restarted) {
      // Handle case where restart might create a new entry with different ID
      if (restarted.id === entry.id) {
        // Same ID - update in place
        const index = entries.value.findIndex(e => e.id === entry.id)
        if (index !== -1) {
          entries.value[index] = restarted
        }
      } else {
        // Different ID - new entry was created
        // Remove the old entry and add the new one
        entries.value = entries.value.filter(e => e.id !== entry.id)
        // Check if WebSocket already added the new entry (race condition)
        const existingIndex = entries.value.findIndex(e => e.id === restarted.id)
        if (existingIndex !== -1) {
          entries.value[existingIndex] = restarted
        } else {
          entries.value.unshift(restarted)
        }
      }
      const projectName = restarted.project?.client
        ? `${restarted.project.client.name} - ${restarted.project.name}`
        : restarted.project?.name || 'Timer'
      api.updateTrayStatus(true, projectName)
      api.showNotification('Timer Started', projectName)
    }
  } catch (err) {
    console.error('Failed to restart entry:', err)
  } finally {
    restartingId.value = null
  }
}

async function openFullApp() {
  try {
    await open(NUXT_URL)
  } catch (err) {
    console.error('Failed to open browser:', err)
  }
}

async function openNewEntryForm() {
  showNewEntryForm.value = true
  newEntryClientId.value = ''
  newEntryProjectId.value = ''
  newEntryDescription.value = ''
  clientProjects.value = []

  try {
    clients.value = await api.getClients()
  } catch (err) {
    console.error('Failed to fetch clients:', err)
  }
}

function closeNewEntryForm() {
  showNewEntryForm.value = false
}

async function openEditEntryForm(entry: TimeEntry) {
  editingEntry.value = entry
  showEditEntryForm.value = true
  editEntryDescription.value = entry.description || ''
  editEntryBillable.value = entry.is_billable

  // Parse date from started_at
  const startDate = new Date(entry.started_at)
  editEntryDate.value = `${startDate.getFullYear()}-${String(startDate.getMonth() + 1).padStart(2, '0')}-${String(startDate.getDate()).padStart(2, '0')}`

  // Parse start time
  editEntryStartTime.value = `${String(startDate.getHours()).padStart(2, '0')}:${String(startDate.getMinutes()).padStart(2, '0')}:${String(startDate.getSeconds()).padStart(2, '0')}`

  // Parse end time if entry is stopped
  if (entry.stopped_at) {
    const endDate = new Date(entry.stopped_at)
    editEntryEndTime.value = `${String(endDate.getHours()).padStart(2, '0')}:${String(endDate.getMinutes()).padStart(2, '0')}:${String(endDate.getSeconds()).padStart(2, '0')}`
  } else {
    editEntryEndTime.value = ''
  }

  // Fetch clients
  try {
    clients.value = await api.getClients()
  } catch (err) {
    console.error('Failed to fetch clients:', err)
  }

  // Set client and project based on entry
  if (entry.project?.client?.id) {
    editEntryClientId.value = entry.project.client.id
    // Load projects for this client
    loadingEditProjects.value = true
    try {
      editClientProjects.value = await api.getClientProjects(entry.project.client.id)
      editEntryProjectId.value = entry.project_id
    } catch (err) {
      console.error('Failed to fetch projects:', err)
    } finally {
      loadingEditProjects.value = false
    }
  } else {
    editEntryClientId.value = ''
    editEntryProjectId.value = ''
    editClientProjects.value = []
  }
}

function closeEditEntryForm() {
  showEditEntryForm.value = false
  editingEntry.value = null
}

async function onEditClientChange() {
  editEntryProjectId.value = ''
  editClientProjects.value = []

  if (editEntryClientId.value) {
    loadingEditProjects.value = true
    try {
      editClientProjects.value = await api.getClientProjects(editEntryClientId.value as number)
    } catch (err) {
      console.error('Failed to fetch projects:', err)
    } finally {
      loadingEditProjects.value = false
    }
  }
}

async function handleSaveEntry() {
  if (!editingEntry.value || !editEntryProjectId.value) return

  savingEntry.value = true
  try {
    // Build started_at datetime - convert local time to ISO string (UTC)
    const startDateTime = new Date(`${editEntryDate.value}T${editEntryStartTime.value}`)
    const startedAt = startDateTime.toISOString()

    // Build stopped_at datetime (only if entry was stopped)
    let stoppedAt: string | null = null
    if (editingEntry.value.stopped_at && editEntryEndTime.value) {
      const endDateTime = new Date(`${editEntryDate.value}T${editEntryEndTime.value}`)
      stoppedAt = endDateTime.toISOString()
    }

    const updatedEntry = await api.updateEntry(editingEntry.value.id, {
      project_id: editEntryProjectId.value as number,
      description: editEntryDescription.value || null,
      started_at: startedAt,
      stopped_at: stoppedAt,
      is_billable: editEntryBillable.value
    })

    if (updatedEntry) {
      // Update entry in local list
      const index = entries.value.findIndex(e => e.id === updatedEntry.id)
      if (index !== -1) {
        entries.value[index] = updatedEntry
      }
      api.showNotification('Entry Updated', updatedEntry.project?.name || 'Time entry updated')
      closeEditEntryForm()
    }
  } catch (err) {
    console.error('Failed to update entry:', err)
  } finally {
    savingEntry.value = false
  }
}

async function onClientChange() {
  newEntryProjectId.value = ''
  clientProjects.value = []

  if (newEntryClientId.value) {
    loadingProjects.value = true
    try {
      clientProjects.value = await api.getClientProjects(newEntryClientId.value as number)
    } catch (err) {
      console.error('Failed to fetch projects:', err)
    } finally {
      loadingProjects.value = false
    }
  }
}

async function handleStartNewEntry() {
  if (!newEntryProjectId.value) return

  startingNewEntry.value = true
  try {
    const newEntry = await api.startTimer(
      newEntryProjectId.value as number,
      newEntryDescription.value || undefined
    )

    if (newEntry) {
      // Check if WebSocket already added this entry (race condition)
      const existingIndex = entries.value.findIndex(e => e.id === newEntry.id)
      if (existingIndex !== -1) {
        entries.value[existingIndex] = newEntry
      } else {
        entries.value.unshift(newEntry)
      }
      const projectName = newEntry.project?.client
        ? `${newEntry.project.client.name} - ${newEntry.project.name}`
        : newEntry.project?.name || 'Timer'
      api.updateTrayStatus(true, projectName)
      api.showNotification('Timer Started', projectName)
      closeNewEntryForm()
    }
  } catch (err) {
    console.error('Failed to start new entry:', err)
  } finally {
    startingNewEntry.value = false
  }
}

function showContextMenu(event: MouseEvent, entry: TimeEntry) {
  event.preventDefault()
  event.stopPropagation()

  const menuWidth = 180
  const menuHeight = 185 // Approximate height of menu
  const windowWidth = window.innerWidth
  const windowHeight = window.innerHeight
  const padding = 8

  // Calculate position, keeping menu within window bounds
  let x = event.clientX
  let y = event.clientY

  // If menu would overflow right edge, position to the left of cursor
  if (x + menuWidth + padding > windowWidth) {
    x = windowWidth - menuWidth - padding
  }

  // If menu would overflow bottom edge, position above cursor
  if (y + menuHeight + padding > windowHeight) {
    y = windowHeight - menuHeight - padding
  }

  // Ensure menu doesn't go off left or top edge
  x = Math.max(padding, x)
  y = Math.max(padding, y)

  contextMenu.value = {
    visible: true,
    x,
    y,
    entry
  }
}

function hideContextMenu() {
  contextMenu.value.visible = false
  contextMenu.value.entry = null
}

async function handleContextEdit() {
  if (!contextMenu.value.entry) return
  const entry = contextMenu.value.entry
  hideContextMenu()
  await openEditEntryForm(entry)
}

async function handleContextStartTimer() {
  if (!contextMenu.value.entry) return
  const entry = contextMenu.value.entry
  hideContextMenu()
  await handleRestartEntry(entry)
}

async function handleContextStopTimer() {
  if (!contextMenu.value.entry) return
  const entry = contextMenu.value.entry
  hideContextMenu()
  await handleStopEntry(entry)
}

async function handleContextDelete() {
  if (!contextMenu.value.entry) return
  const entry = contextMenu.value.entry
  hideContextMenu()

  deletingId.value = entry.id
  try {
    await api.deleteEntry(entry.id)
    entries.value = entries.value.filter(e => e.id !== entry.id)
    api.showNotification('Entry Deleted', entry.project?.name || 'Timer entry deleted')
  } catch (err) {
    console.error('Failed to delete entry:', err)
  } finally {
    deletingId.value = null
  }
}

async function handleContextViewInApp() {
  if (!contextMenu.value.entry) return
  const entryId = contextMenu.value.entry.id
  hideContextMenu()
  try {
    await open(`${NUXT_URL}/time-tracking?entry=${entryId}`)
  } catch (err) {
    console.error('Failed to open browser:', err)
  }
}

async function refresh() {
  loading.value = true
  error.value = null
  try {
    // Fetch entries and settings in parallel
    const [, settings] = await Promise.all([
      fetchEntries(),
      api.getSettings()
    ])
    userSettings.value = settings
  } catch (err) {
    error.value = 'Failed to connect to server'
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await refresh()

  // Initialize tray timer if there's already a running entry
  const runningEntry = entries.value.find(e => !e.stopped_at)
  if (runningEntry) {
    const sessionStart = new Date(runningEntry.resumed_at || runningEntry.started_at)
    const currentSessionMs = Math.max(0, Date.now() - sessionStart.getTime())
    const currentSessionSecs = Math.floor(currentSessionMs / 1000)
    const accumulatedSecs = Math.floor((runningEntry.duration_minutes || 0) * 60)
    api.updateTrayTimer(accumulatedSecs + currentSessionSecs)
  }

  // Update current time frequently for accurate timer display
  // Using 100ms ensures we're within 100ms of the true second change
  currentTime.value = Date.now()
  let lastTrayUpdateSecond = -1

  timerInterval = window.setInterval(() => {
    currentTime.value = Date.now()

    // Update tray timer display every second when there's a running timer
    const runningEntry = entries.value.find(e => !e.stopped_at)
    if (runningEntry) {
      const sessionStart = new Date(runningEntry.resumed_at || runningEntry.started_at)
      const currentSessionMs = Math.max(0, currentTime.value - sessionStart.getTime())
      const currentSessionSecs = Math.floor(currentSessionMs / 1000)
      const accumulatedSecs = Math.floor((runningEntry.duration_minutes || 0) * 60)
      const totalSecs = accumulatedSecs + currentSessionSecs

      // Only update tray if second changed (to avoid excessive calls)
      if (totalSecs !== lastTrayUpdateSecond) {
        lastTrayUpdateSecond = totalSecs
        api.updateTrayTimer(totalSecs)
      }
    }
  }, 100)

  // WebSocket listeners
  unlistenWsTimerStarted = await listen<any>('ws-timer-started', (event) => {
    const entry = event.payload as TimeEntry
    console.log('WebSocket: Timer started', entry.id)

    // Check if entry exists by ID (comparing as numbers to avoid type issues)
    const entryId = Number(entry.id)
    const index = entries.value.findIndex(e => Number(e.id) === entryId)

    if (index !== -1) {
      // Entry exists - update it in place
      entries.value[index] = entry
    } else {
      // Entry doesn't exist - add it, but double-check to prevent duplicates
      // This handles race conditions where the entry might have been added between checks
      if (!entries.value.some(e => Number(e.id) === entryId)) {
        entries.value.unshift(entry)
      }
    }
    const projectName = entry.project?.client
      ? `${entry.project.client.name} - ${entry.project.name}`
      : entry.project?.name || 'Timer'
    api.updateTrayStatus(true, projectName)
  })

  unlistenWsTimerStopped = await listen<any>('ws-timer-stopped', (event) => {
    const entry = event.payload as TimeEntry
    console.log('WebSocket: Timer stopped', entry.id)

    // Compare as numbers to avoid type issues
    const entryId = Number(entry.id)
    const index = entries.value.findIndex(e => Number(e.id) === entryId)
    if (index !== -1) {
      entries.value[index] = entry
    }
    // Check if there's still a running timer
    const hasRunning = entries.value.some(e => !e.stopped_at)
    if (!hasRunning) {
      api.updateTrayStatus(false)
      api.clearTrayTimer() // Reset menu bar to just show icon
    }
  })

  unlistenWsTimerDeleted = await listen<any>('ws-timer-deleted', (event) => {
    const entryId = event.payload as number
    console.log('WebSocket: Timer deleted', entryId)
    entries.value = entries.value.filter(e => e.id !== entryId)
  })

  // Listen for window focus to refresh settings when widget is shown
  const appWindow = getCurrentWindow()
  unlistenWindowFocus = await appWindow.onFocusChanged(async ({ payload: focused }) => {
    if (focused) {
      // Silently refresh settings when window gains focus
      try {
        userSettings.value = await api.getSettings()
      } catch (err) {
        console.error('Failed to refresh settings on focus:', err)
      }
    }
  })
})

onUnmounted(() => {
  if (timerInterval) clearInterval(timerInterval)
  if (unlistenWsTimerStarted) unlistenWsTimerStarted()
  if (unlistenWsTimerStopped) unlistenWsTimerStopped()
  if (unlistenWsTimerDeleted) unlistenWsTimerDeleted()
  if (unlistenWindowFocus) unlistenWindowFocus()
})
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body {
  margin: 0;
  padding: 0;
  background: transparent;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

#app {
  width: 100%;
  height: 100%;
}
</style>

<style scoped>
.menubar-popup {
  width: calc(100% - 16px);
  max-height: calc(100vh - 16px);
  margin: 8px;
  background: #ffffff;
  border-radius: 0.5rem;
  box-shadow: 0 4px 4px rgba(0, 0, 0, 0.08), 0 0 1px rgba(0, 0, 0, 0.1);
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif;
  overflow: hidden;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}

/* Loading */
.loading {
  padding: 40px 20px;
  text-align: center;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid #e0e0e0;
  border-top-color: #fa5d00;
  border-radius: 50%;
  margin: 0 auto 12px;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  color: #888;
  font-size: 13px;
}

/* Error */
.error-state {
  padding: 30px 20px;
  text-align: center;
}

.error-icon {
  font-size: 28px;
  margin-bottom: 8px;
}

.error-text {
  color: #dc3545;
  font-size: 13px;
  margin-bottom: 16px;
}

.retry-button {
  padding: 8px 16px;
  background: #fa5d00;
  border: none;
  border-radius: 6px;
  color: white;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

/* Date Header */
.date-header {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  padding: 10px 14px;
  background: #f1f0ee;
}

.date-text {
  font-size: 13px;
  font-weight: 600;
  color: #1a202c;
  text-align: center;
}

.date-header-left {
  display: flex;
  align-items: center;
  gap: 6px;
}

.nav-arrow {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  color: #666;
  border-radius: 4px;
  transition: all 0.15s;
}

.nav-arrow:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #1a202c;
}

.nav-arrow svg {
  width: 16px;
  height: 16px;
}

.calendar-icon {
  width: 22px;
  height: 22px;
  color: #1a202c;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  transition: opacity 0.15s;
  position: relative;
  justify-self: end;
}

.calendar-icon:hover {
  opacity: 0.7;
}

.calendar-icon svg {
  width: 22px;
  height: 22px;
}

.calendar-day {
  position: absolute;
  font-size: 9px;
  font-weight: 700;
  color: #1a202c;
  top: 8px;
}

/* Week Bar */
.week-bar {
  display: flex;
  background: #f8f8f8;
  border-bottom: 1px solid #e8e8e8;
  padding: 8px;
}

.day-button {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 6px 2px;
  background: none;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
}

.day-button:hover {
  background: rgba(0, 0, 0, 0.05);
}

.day-button.active {
  background: #f1f0ee;
}

.day-button.active .day-letter,
.day-button.active .day-total {
  color: #1a202c;
}

.day-button.today:not(.active) .day-letter {
  color: #56c97b;
  font-weight: 700;
}

.day-letter {
  font-size: 11px;
  font-weight: 600;
  color: #666;
  margin-bottom: 2px;
}

.day-total {
  font-size: 10px;
  color: #999;
  min-height: 14px;
}

/* Day Header */
.day-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  background: #fafafa;
  border-bottom: 1px solid #e8e8e8;
}

.day-title {
  font-size: 13px;
  font-weight: 600;
  color: #333;
}

.day-duration {
  font-size: 13px;
  font-weight: 600;
  color: #1a202c;
  font-variant-numeric: tabular-nums;
}

.day-revenue {
  font-size: 13px;
  font-weight: 600;
  color: #15803d;
  font-variant-numeric: tabular-nums;
  background: #f0fdf4;
  border: 1px solid #bbf7d0;
  border-radius: 6px;
  padding: 2px 8px;
}

/* Entries List */
.entries-list {
  min-height: 150px;
  max-height: 280px;
  overflow-y: auto;
}

.no-entries {
  padding: 30px 14px;
  text-align: center;
  color: #999;
  font-size: 13px;
}

.entry-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  border-bottom: 1px solid #f0f0f0;
  transition: background 0.15s;
}

.entry-item:hover {
  background: #fafafa;
}

.entry-item.running {
  background: #f0faf3;
}

.entry-info {
  flex: 1;
  min-width: 0;
  margin-right: 10px;
}

.entry-client {
  font-size: 10px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin-bottom: 1px;
}

.entry-project {
  font-size: 13px;
  font-weight: 500;
  color: #333;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-description {
  font-size: 11px;
  color: #666;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.entry-duration {
  font-size: 13px;
  font-weight: 500;
  color: #333;
  font-variant-numeric: tabular-nums;
  min-width: 50px;
  text-align: right;
}

.entry-duration.running {
  color: #1a202c;
}

.action-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  transition: all 0.15s;
}

.play-btn {
  background: #f0f0f0;
  color: #666;
}

.play-btn:hover:not(:disabled) {
  background: #56c97b;
  color: white;
}

.stop-btn {
  background: #ef4444;
  color: white;
}

.stop-btn:hover:not(:disabled) {
  background: #dc2626;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.play-btn .spinner {
  border-color: rgba(0,0,0,0.2);
  border-top-color: #666;
}

/* Footer */
.footer {
  display: flex;
  gap: 10px;
  padding: 10px 14px;
  background: #f8f8f8;
  border-top: 1px solid #e8e8e8;
}

.footer-button {
  flex: 1;
  padding: 8px;
  background: white;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  color: #333;
  cursor: pointer;
  transition: all 0.15s;
}

.footer-button:hover {
  background: #f5f5f5;
  border-color: #1a202c;
}

.footer-button-add {
  width: 34px;
  height: 34px;
  padding: 0;
  background: white;
  border: 1px solid #ddd;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.footer-button-add svg {
  width: 18px;
  height: 18px;
  color: #1a202c;
}

.footer-button-add:hover {
  background: #f5f5f5;
  border-color: #1a202c;
}

/* New Entry Form Header */
.new-entry-header {
  justify-content: space-between;
}

.close-form-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: none;
  color: #666;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: background 0.15s;
  padding: 0;
}

.close-form-btn:hover {
  background: rgba(0, 0, 0, 0.1);
}

.close-form-btn svg {
  width: 16px;
  height: 16px;
}

/* New Entry Form (inline) */
.new-entry-form {
  padding: 16px 14px 8px;
  flex: 1;
  overflow-y: auto;
  max-height: 250px;
}

.form-group {
  margin-bottom: 14px;
}

.form-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin-bottom: 6px;
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  font-size: 13px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: white;
  color: #1a202c;
  transition: border-color 0.15s;
  font-family: inherit;
}

.form-input:focus {
  outline: none;
  border-color: #1a202c;
}

.form-select {
  width: 100%;
  padding: 10px 32px 10px 12px;
  font-size: 13px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: white;
  color: #1a202c;
  transition: border-color 0.15s;
  font-family: inherit;
  cursor: pointer;
  /* Custom dropdown arrow */
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23666' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  background-size: 12px;
}

.form-select:focus {
  outline: none;
  border-color: #1a202c;
}

.form-select:disabled {
  background-color: #f5f5f5;
  color: #999;
  cursor: not-allowed;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23999' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
}

.start-timer-btn {
  width: 100%;
  padding: 12px;
  background: #1a202c;
  border: 1px solid #1a202c;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: all 0.15s;
}

.start-timer-btn:hover:not(:disabled) {
  background: #2d3748;
}

.start-timer-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Form Button Container (fixed at bottom) */
.form-button-container {
  padding: 8px 14px 16px;
  background: white;
  border-top: 1px solid #f0f0f0;
}

/* Edit Entry Form (inline) */
.edit-entry-form {
  padding: 16px 14px 8px;
  flex: 1;
  overflow-y: auto;
  max-height: 280px;
}

.form-textarea {
  width: 100%;
  padding: 10px 12px;
  font-size: 13px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: white;
  color: #1a202c;
  transition: border-color 0.15s;
  font-family: inherit;
  resize: none;
}

.form-textarea:focus {
  outline: none;
  border-color: #1a202c;
}

.form-row {
  display: flex;
  gap: 12px;
}

.form-group-half {
  flex: 1;
  margin-bottom: 14px;
}

.form-group-inline {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}

.form-checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: #1a202c;
}

.form-label-inline {
  font-size: 13px;
  color: #333;
  cursor: pointer;
}

.save-entry-btn {
  width: 100%;
  padding: 12px;
  background: #1a202c;
  border: 1px solid #1a202c;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  transition: all 0.15s;
}

.save-entry-btn:hover:not(:disabled) {
  background: #2d3748;
}

.save-entry-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

</style>

<!-- Unscoped styles for teleported context menu -->
<style>
.context-menu {
  position: fixed;
  min-width: 180px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(0, 0, 0, 0.08);
  padding: 4px 0;
  z-index: 9999;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif;
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 14px;
  background: none;
  border: none;
  text-align: left;
  font-size: 13px;
  color: #333;
  cursor: pointer;
  transition: background 0.15s;
}

.context-menu-item:hover:not(:disabled) {
  background: #f5f5f5;
}

.context-menu-item:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.context-menu-item-danger {
  color: #dc3545;
}

.context-menu-item-danger:hover:not(:disabled) {
  background: #fef2f2;
}

.context-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.context-menu-separator {
  height: 1px;
  background: #e8e8e8;
  margin: 4px 0;
}
</style>
