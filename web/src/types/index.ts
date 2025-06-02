/**
 * Type definitions for the voice recorder application
 */

// Note interface for individual recordings (legacy, for compatibility)
export interface Note {
  id: string;
  title: string;
  content: string;
  tag: string;
  date: string;
  duration: string;
  image: string;
  audioUrl?: string;
  transcript?: string;
  notes?: NoteSection[];
}

// Note section for structured content (legacy)
export interface NoteSection {
  section: string;
  content: string[];
}

// Backend API types
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

// Recording state for the recorder component


// Audio player state
export interface AudioPlayerState {
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
}

// Filter options for notes
export interface FilterOptions {
  category: string;
  searchQuery: string;
  sortBy: 'date' | 'title' | 'duration';
  sortOrder: 'asc' | 'desc';
}

// App configuration
export interface AppConfig {
  maxRecordingDuration: number; // in seconds
  supportedAudioFormats: string[];
  autoSave: boolean;
  theme: 'light' | 'dark' | 'auto';
}