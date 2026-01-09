import { invoke } from '@tauri-apps/api/core'

export interface Client {
  id: number
  name: string
  hourly_rate: number | null
}

export interface UserSettings {
  show_live_revenue?: boolean
}

export interface Project {
  id: number
  name: string
  client: Client | null
}

export interface TimeEntry {
  id: number
  project_id: number
  project: Project | null
  description: string | null
  started_at: string
  stopped_at: string | null
  resumed_at: string | null
  duration_minutes: number | null
  is_billable: boolean
}

export interface UpdateEntryData {
  project_id: number
  description: string | null
  started_at: string
  stopped_at: string | null
  is_billable: boolean
}

class ApiService {
  async updateTrayStatus(isRunning: boolean, projectName?: string): Promise<void> {
    try {
      await invoke('update_tray_status', {
        isRunning,
        projectName: projectName || null
      })
    } catch (err) {
      console.error('Failed to update tray status:', err)
    }
  }

  async updateTrayTimer(elapsedSeconds: number): Promise<void> {
    try {
      await invoke('update_tray_timer', { elapsedSeconds: Math.floor(elapsedSeconds) })
    } catch (err) {
      console.error('Failed to update tray timer:', err)
    }
  }

  async clearTrayTimer(): Promise<void> {
    try {
      await invoke('clear_tray_timer')
    } catch (err) {
      console.error('Failed to clear tray timer:', err)
    }
  }

  async syncTimerState(timer: TimeEntry | null, firstProjectId: number | null): Promise<void> {
    try {
      await invoke('sync_timer_state', {
        timer,
        firstProjectId
      })
    } catch (err) {
      console.error('Failed to sync timer state:', err)
    }
  }

  async showNotification(title: string, body: string): Promise<void> {
    try {
      await invoke('show_notification', { title, body })
    } catch (err) {
      console.error('Failed to show notification:', err)
    }
  }

  async getActiveTimer(): Promise<TimeEntry | null> {
    try {
      return await invoke<TimeEntry | null>('get_active_timer')
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to get active timer:', error)
      return null
    }
  }

  async startTimer(projectId: number, description?: string): Promise<TimeEntry | null> {
    try {
      return await invoke<TimeEntry>('start_timer', {
        projectId,
        description: description || null
      })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to start timer:', error)
      throw error
    }
  }

  async stopTimer(timerId: number): Promise<TimeEntry | null> {
    try {
      return await invoke<TimeEntry>('stop_timer', { timerId })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to stop timer:', error)
      throw error
    }
  }

  async getRecentProjects(): Promise<Project[]> {
    try {
      return await invoke<Project[]>('get_recent_projects')
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to get recent projects:', error)
      return []
    }
  }

  async getTimeEntries(): Promise<TimeEntry[]> {
    try {
      return await invoke<TimeEntry[]>('get_time_entries')
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to get time entries:', error)
      return []
    }
  }

  async restartTimer(timerId: number): Promise<TimeEntry | null> {
    try {
      return await invoke<TimeEntry>('restart_timer', { timerId })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to restart timer:', error)
      throw error
    }
  }

  async deleteEntry(entryId: number): Promise<void> {
    try {
      await invoke('delete_entry', { entryId })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to delete entry:', error)
      throw error
    }
  }

  async getClients(): Promise<Client[]> {
    try {
      return await invoke<Client[]>('get_clients')
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to get clients:', error)
      return []
    }
  }

  async getClientProjects(clientId: number): Promise<Project[]> {
    try {
      return await invoke<Project[]>('get_client_projects', { clientId })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to get client projects:', error)
      return []
    }
  }

  async updateEntry(entryId: number, data: UpdateEntryData): Promise<TimeEntry | null> {
    try {
      return await invoke<TimeEntry>('update_entry', { entryId, data })
    } catch (error: any) {
      if (error?.includes?.('Not authenticated')) {
        throw new Error('AUTH_REQUIRED')
      }
      console.error('Failed to update entry:', error)
      throw error
    }
  }

  async getSettings(): Promise<UserSettings> {
    try {
      return await invoke<UserSettings>('get_settings')
    } catch (error: any) {
      console.error('Failed to get settings:', error)
      return {}
    }
  }
}

export const api = new ApiService()
