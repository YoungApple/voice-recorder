import axios from 'axios';

/**
 * API service for communicating with the backend
 */

export interface VoiceSession {
  id: string;
  timestamp: string;
  audio_file_path: string;
  transcript?: string;
  analysis?: AnalysisResult;
  title: string;
  duration_ms: number;
  audio_url?: string;
}

export interface AnalysisResult {
  title: string;
  ideas: string[];
  tasks: Task[];
  structured_notes: StructuredNote[];
  summary: string;
}

export interface Task {
  title: string;
  description?: string;
  priority: 'Low' | 'Medium' | 'High' | 'Urgent';
  due_date?: string;
}

export interface StructuredNote {
  title: string;
  content: string;
  tags: string[];
  note_type: 'Meeting' | 'Brainstorm' | 'Decision' | 'Action' | 'Reference';
  updated_at: string;
  created_at: string;
}

export interface ApiResponse<T> {
  data: T;
  message?: string;
  error?: string;
}

const API_BASE_URL = 'http://localhost:3000/api';

// Create axios instance with default configuration
const apiClient = axios.create({
  baseURL: API_BASE_URL,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add response interceptor for error handling
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response) {
      // Server responded with error status
      throw new Error(`API Error: ${error.response.status} - ${error.response.statusText}`);
    } else if (error.request) {
      // Request was made but no response received
      throw new Error('Network Error: No response from server');
    } else {
      // Something else happened
      throw new Error(`Request Error: ${error.message}`);
    }
  }
);

/**
 * Fetch all voice sessions from the backend
 */
export async function fetchSessions(params?: {
  search?: string;
  sort_by?: string;
  sort_order?: string;
  limit?: number;
  offset?: number;
}): Promise<VoiceSession[]> {
  const response = await apiClient.get<ApiResponse<VoiceSession[]>>('/sessions', {
    params,
  });
  return response.data.data;
}

/**
 * Fetch a specific voice session by ID
 */
export async function fetchSession(id: string): Promise<VoiceSession> {
  const response = await apiClient.get<ApiResponse<VoiceSession>>(`/sessions/${id}`);
  return response.data.data;
}

/**
 * Delete a voice session
 */
export async function deleteSession(id: string): Promise<void> {
  await apiClient.delete(`/sessions/${id}`);
}

/**
 * Start a new recording session
 */
export async function startRecording(): Promise<string> {
  const response = await apiClient.post<ApiResponse<void>>('/record/start');
  // For now, return a generated session ID
  // In a real implementation, the backend should return the session ID
  return `session_${Date.now()}`;
}

/**
 * Stop recording
 */
export async function stopRecording(): Promise<void> {
  await apiClient.post('/record/stop');
}

/**
 * Get recording status
 */
export async function getRecordingStatus(): Promise<boolean> {
  const response = await apiClient.get<ApiResponse<boolean>>('/record/status');
  return response.data.data;
}

/**
 * Get audio URL for a session
 */
export function getAudioUrl(sessionId: string): string {
  return `${API_BASE_URL}/sessions/${sessionId}/audio`;
}

/**
 * Get transcript for a session
 */
export async function fetchTranscript(id: string): Promise<string> {
  const response = await apiClient.get<ApiResponse<string>>(`/sessions/${id}/transcript`);
  return response.data.data;
}

/**
 * Get analysis for a session
 */
export async function fetchAnalysis(id: string): Promise<AnalysisResult | null> {
  const response = await apiClient.get<ApiResponse<AnalysisResult | null>>(`/sessions/${id}/analysis`);
  return response.data.data;
}

/**
 * Upload audio file and create a new session
 * This function handles the complete flow of uploading an audio file,
 * processing it on the backend, and returning the created session
 */
export async function uploadAudioFile(audioFile: File): Promise<VoiceSession> {
  const formData = new FormData();
  formData.append('audio', audioFile);
  
  const response = await apiClient.post<ApiResponse<VoiceSession>>(
    '/sessions/upload',
    formData,
    {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
      // Set a longer timeout for audio processing
      timeout: 60000, // 60 seconds
    }
  );
  
  return response.data.data;
}