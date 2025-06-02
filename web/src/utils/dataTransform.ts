/**
 * Data transformation utilities for converting between frontend and backend types
 */

import type { Note, VoiceSession, AnalysisResult } from '../types';
import { formatTime } from './index';

/**
 * Convert VoiceSession from backend to Note for frontend display
 */
export function voiceSessionToNote(session: VoiceSession): Note {
  // Generate a default image based on the note type or use a fallback
  const getImageForNoteType = (analysis?: AnalysisResult): string => {
    if (!analysis?.structured_notes?.length) {
      return "https://images.unsplash.com/photo-1589254065878-42c9da997008?w=800&q=80";
    }
    
    const noteType = analysis.structured_notes[0].note_type;
    switch (noteType) {
      case 'Meeting':
        return "https://images.unsplash.com/photo-1517180102446-f3ece451e9d8?w=800&q=80";
      case 'Brainstorm':
        return "https://images.unsplash.com/photo-1505740420928-5e560c06d30e?w=800&q=80";
      case 'Decision':
        return "https://images.unsplash.com/photo-1488190211105-8b0e65b80b4e?w=800&q=80";
      case 'Action':
        return "https://images.unsplash.com/photo-1517180102446-f3ece451e9d8?w=800&q=80";
      case 'Reference':
        return "https://images.unsplash.com/photo-1505740420928-5e560c06d30e?w=800&q=80";
      default:
        return "https://images.unsplash.com/photo-1589254065878-42c9da997008?w=800&q=80";
    }
  };

  // Get tag from analysis or use default
  const getTag = (analysis?: AnalysisResult): string => {
    if (!analysis?.structured_notes?.length) {
      return "Ideas";
    }
    
    const noteType = analysis.structured_notes[0].note_type;
    switch (noteType) {
      case 'Meeting':
        return "Meeting";
      case 'Brainstorm':
        return "Ideas";
      case 'Decision':
        return "Decision";
      case 'Action':
        return "Tasks";
      case 'Reference':
        return "Reference";
      default:
        return "Ideas";
    }
  };

  // Convert structured notes to legacy note sections
  const convertToNoteSections = (analysis?: AnalysisResult) => {
    if (!analysis) return undefined;
    
    const sections = [];
    
    if (analysis.ideas.length > 0) {
      sections.push({
        section: "Ideas",
        content: analysis.ideas
      });
    }
    
    if (analysis.tasks.length > 0) {
      sections.push({
        section: "Tasks",
        content: analysis.tasks.map(task => 
          `${task.title}${task.description ? ': ' + task.description : ''} (${task.priority})`
        )
      });
    }
    
    if (analysis.structured_notes.length > 0) {
      analysis.structured_notes.forEach(note => {
        sections.push({
          section: note.title,
          content: [note.content]
        });
      });
    }
    
    return sections.length > 0 ? sections : undefined;
  };

  return {
    id: session.id,
    title: session.title,
    content: session.analysis?.summary || session.transcript || "No content available",
    tag: getTag(session.analysis),
    date: new Date(session.timestamp).toISOString().split('T')[0],
    duration: formatTime(Math.floor(session.duration_ms / 1000)),
    image: getImageForNoteType(session.analysis),
    audioUrl: session.audio_url,
    transcript: session.transcript,
    notes: convertToNoteSections(session.analysis)
  };
}

/**
 * Convert array of VoiceSessions to Notes
 */
export function voiceSessionsToNotes(sessions: VoiceSession[]): Note[] {
  return sessions.map(voiceSessionToNote);
}

/**
 * Get unique categories from voice sessions
 */
export function getCategoriesFromSessions(sessions: VoiceSession[]): string[] {
  const categories = new Set<string>();
  
  sessions.forEach(session => {
    if (session.analysis?.structured_notes?.length) {
      const noteType = session.analysis.structured_notes[0].note_type;
      switch (noteType) {
        case 'Meeting':
          categories.add("Meeting");
          break;
        case 'Brainstorm':
          categories.add("Ideas");
          break;
        case 'Decision':
          categories.add("Decision");
          break;
        case 'Action':
          categories.add("Tasks");
          break;
        case 'Reference':
          categories.add("Reference");
          break;
        default:
          categories.add("Ideas");
      }
    } else {
      categories.add("Ideas");
    }
  });
  
  return Array.from(categories).sort();
}