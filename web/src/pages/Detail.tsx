import React, { useState, useRef, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  ArrowLeftIcon,
  PlayIcon,
  PauseIcon,
  ShareIcon,
  TrashIcon,
  DocumentArrowDownIcon,
  LightBulbIcon,
  CheckCircleIcon,
  ChatBubbleLeftRightIcon,
  LightBulbIcon as BrainstormIcon,
  DocumentCheckIcon,
  DocumentIcon,
  BookOpenIcon,
  ClockIcon,
  ExclamationTriangleIcon,
  FireIcon,
} from "@heroicons/react/24/outline";
import { copyToClipboard, formatDate } from "../utils";
import { fetchSession, deleteSession, getAudioUrl, fetchTranscript, fetchAnalysis } from "../services/api";
import type { Note, VoiceSession } from "../types";
import { voiceSessionToNote } from "../utils/dataTransform";

// Removed mock data - now using real API data

const Detail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const audioRef = useRef<HTMLAudioElement>(null);
  
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [session, setSession] = useState<VoiceSession | null>(null);
  const [note, setNote] = useState<Note | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  
  // Load session data on component mount
  useEffect(() => {
    if (id) {
      loadSession(id);
    }
  }, [id]);
  
  // Convert session to note format when session changes
  useEffect(() => {
    if (session) {
      const convertedNote = voiceSessionToNote(session);
      setNote(convertedNote);
      
      // Load audio URL
      loadAudioUrl(session.id);
    }
  }, [session]);
  
  const loadSession = async (sessionId: string) => {
    try {
      setLoading(true);
      setError(null);
      const sessionData = await fetchSession(sessionId);
      setSession(sessionData);
    } catch (err) {
      console.error('Failed to load session:', err);
      setError('Failed to load recording details. Please try again.');
    } finally {
      setLoading(false);
    }
  };
  
  const loadAudioUrl = async (sessionId: string) => {
    try {
      const url = await getAudioUrl(sessionId);
      setAudioUrl(url);
    } catch (err) {
      console.error('Failed to load audio URL:', err);
    }
  };
  
  // 实际开发中可根据 id 拉取后端数据
  const detail = note;

  // Audio control functions
  const togglePlayback = () => {
    if (audioRef.current) {
      if (isPlaying) {
        audioRef.current.pause();
      } else {
        audioRef.current.play();
      }
      setIsPlaying(!isPlaying);
    }
  };

  const handleTimeUpdate = () => {
    if (audioRef.current) {
      setCurrentTime(audioRef.current.currentTime);
    }
  };

  const handleLoadedMetadata = () => {
    if (audioRef.current) {
      setDuration(audioRef.current.duration);
    }
  };

  const handleAudioEnded = () => {
    setIsPlaying(false);
    setCurrentTime(0);
  };

  const formatTime = (time: number) => {
    const minutes = Math.floor(time / 60);
    const seconds = Math.floor(time % 60);
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  };

  const handleExport = async () => {
    try {
      const exportData = {
        title: detail?.title,
        date: detail?.date,
        duration: detail?.duration,
        transcript: detail?.transcript,
        notes: detail?.notes
      };
      
      const blob = new Blob([JSON.stringify(exportData, null, 2)], {
        type: 'application/json'
      });
      
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${detail?.title.replace(/\s+/g, '_')}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Export failed:', error);
    }
  };

  const handleDelete = async () => {
    if (window.confirm('Are you sure you want to delete this recording?')) {
      try {
        if (id) {
          await deleteSession(id);
          navigate('/');
        }
      } catch (err) {
        console.error('Failed to delete session:', err);
        setError('Failed to delete recording. Please try again.');
      }
    }
  };

  const handleShare = async () => {
    try {
      const shareText = `${detail?.title}\n\n${detail?.transcript}`;
      
      if (navigator.share) {
        await navigator.share({
          title: detail?.title,
          text: shareText
        });
      } else {
        const success = await copyToClipboard(shareText);
        if (success) {
          alert('Content copied to clipboard!');
        } else {
          console.error('Failed to copy to clipboard');
        }
      }
    } catch (error) {
      console.error('Share failed:', error);
    }
  };

  // Loading state
  if (loading) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading recording details...</p>
        </div>
      </div>
    );
  }
  
  // Error state
  if (error) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <div className="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md">
            <svg className="w-12 h-12 text-red-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
            <h2 className="text-xl font-bold text-gray-900 mb-2">Error Loading Recording</h2>
            <p className="text-gray-600 mb-4">{error}</p>
            <div className="space-x-3">
              <button
                onClick={() => id && loadSession(id)}
                className="bg-indigo-600 text-white px-4 py-2 rounded-lg hover:bg-indigo-700 transition-colors"
              >
                Retry
              </button>
              <button
                onClick={() => navigate('/')}
                className="bg-gray-300 text-gray-700 px-4 py-2 rounded-lg hover:bg-gray-400 transition-colors"
              >
                Back to Home
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }
  
  // Not found state
  if (!detail) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <h2 className="text-2xl font-bold text-gray-900 mb-2">Recording not found</h2>
          <p className="text-gray-600 mb-4">The recording you're looking for doesn't exist.</p>
          <button
            onClick={() => navigate('/')}
            className="bg-indigo-600 text-white px-4 py-2 rounded-lg hover:bg-indigo-700 transition-colors"
          >
            Back to Home
          </button>
        </div>
      </div>
    );
  }

  return (
       <div className="min-h-screen bg-gradient-to-br from-pink-100 to-blue-100">
        <header className="sticky top-0 z-10 flex items-center justify-between px-4 sm:px-6 py-5 bg-white/80 backdrop-blur-md shadow-sm">
          <div className="flex items-center">
            <button 
              onClick={() => navigate(-1)} 
              className="mr-3 p-2.5 text-gray-500 hover:text-blue-600 hover:bg-gray-100 rounded-full transition-colors duration-150"
              title="Go back"
            >
              <ArrowLeftIcon className="h-5 w-5" />
            </button>
            <h1 className="text-xl font-semibold text-gray-800">Note Detail</h1>
          </div>
          <div className="flex items-center gap-2">
            <button 
              onClick={handleShare}
              className="p-2.5 text-gray-500 hover:text-blue-600 hover:bg-gray-100 rounded-full transition-colors duration-150"
              title="Share"
            >
              <ShareIcon className="h-5 w-5" />
            </button>
          </div>
        </header>
      <main className="max-w-3xl mx-auto p-4 sm:p-6">
          <div className="bg-white rounded-2xl shadow-xl p-6 sm:p-8 mb-8">
            {/* Title */}
            <div className="mb-8">
              <h1 className="text-3xl font-bold text-gray-800 mb-3">{detail.title}</h1>
              <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-gray-500">
                <span className="flex items-center"><ArrowLeftIcon className="h-4 w-4 mr-1.5 text-gray-400" /> {formatDate(detail.date)}</span>
                <span className="flex items-center"><PlayIcon className="h-4 w-4 mr-1.5 text-gray-400" /> {detail.duration}</span>
                <span className="px-2.5 py-1 bg-purple-100 text-purple-700 rounded-full text-xs font-medium">{detail.tag}</span>
              </div>
            </div>

            {/* Custom Audio Player - Glassmorphism Design */}
            <div className="mb-8 relative">
              {/* Background with gradient */}
              <div className="absolute inset-0 bg-gradient-to-br from-blue-400/20 via-purple-400/20 to-pink-400/20 rounded-2xl blur-xl"></div>
              
              {/* Main player container with glassmorphism effect */}
              <div className="relative backdrop-blur-md bg-white/30 border border-white/20 rounded-2xl p-6 shadow-xl">
                <div className="flex items-center gap-6">
                  {/* Play/Pause Button with enhanced design */}
                  <div className="relative">
                    <div className="absolute inset-0 bg-gradient-to-br from-blue-500 to-purple-600 rounded-full blur-lg opacity-50"></div>
                    <button 
                      onClick={togglePlayback}
                      className="relative bg-gradient-to-br from-blue-500 to-purple-600 hover:from-blue-600 hover:to-purple-700 text-white p-4 rounded-full transition-all duration-300 shadow-lg hover:shadow-xl transform hover:scale-110 backdrop-blur-sm"
                      title={isPlaying ? "Pause" : "Play"}
                    >
                      {isPlaying ? <PauseIcon className="h-7 w-7" /> : <PlayIcon className="h-7 w-7 ml-0.5" />}
                    </button>
                  </div>
                  
                  {/* Progress section */}
                  <div className="flex-1">
                    {/* Custom progress bar */}
                    <div className="relative mb-3">
                      <div className="h-2 bg-white/20 rounded-full backdrop-blur-sm border border-white/10">
                        <div 
                          className="h-full bg-gradient-to-r from-blue-500 to-purple-600 rounded-full transition-all duration-150 shadow-sm"
                          style={{ width: `${duration ? (currentTime / duration) * 100 : 0}%` }}
                        ></div>
                      </div>
                      <input 
                        type="range"
                        min="0"
                        max={duration || 0}
                        value={currentTime}
                        onChange={(e) => {
                          if (audioRef.current) {
                            audioRef.current.currentTime = Number(e.target.value);
                            setCurrentTime(Number(e.target.value));
                          }
                        }}
                        className="absolute inset-0 w-full h-2 opacity-0 cursor-pointer"
                      />
                    </div>
                    
                    {/* Time display */}
                    <div className="flex justify-between items-center">
                      <span className="text-sm font-medium text-gray-700 bg-white/40 px-2 py-1 rounded-lg backdrop-blur-sm">
                        {formatTime(currentTime)}
                      </span>
                      <div className="flex items-center gap-2">
                        {/* Audio visualizer dots */}
                        <div className="flex items-center gap-1">
                          {[...Array(5)].map((_, i) => (
                            <div 
                              key={i}
                              className={`w-1 bg-gradient-to-t from-blue-500 to-purple-600 rounded-full transition-all duration-300 ${
                                isPlaying ? 'animate-pulse' : ''
                              }`}
                              style={{ 
                                height: isPlaying ? `${Math.random() * 16 + 8}px` : '8px',
                                animationDelay: `${i * 0.1}s`
                              }}
                            ></div>
                          ))}
                        </div>
                      </div>
                      <span className="text-sm font-medium text-gray-700 bg-white/40 px-2 py-1 rounded-lg backdrop-blur-sm">
                        {formatTime(duration)}
                      </span>
                    </div>
                  </div>
                </div>
                
                {/* Audio element */}
                <audio 
                  ref={audioRef} 
                  src={audioUrl || "/audio/sample.mp3"} // Fallback for example
                  onTimeUpdate={handleTimeUpdate}
                  onLoadedMetadata={handleLoadedMetadata}
                  onEnded={handleAudioEnded}
                  className="hidden"
                />
              </div>
            </div>

            {/* Transcript Section */}
            <div className="mb-8">
              <div className="flex items-center space-x-3 mb-6">
                <div className="p-2 bg-gradient-to-br from-yellow-100 to-orange-100 rounded-xl">
                    <DocumentIcon className="w-6 h-6 text-yellow-600" />
                </div>
                <h2 className="text-xl font-semibold text-gray-700 mb">Transcript</h2>
              </div>
              <div className="bg-slate-50 p-4 rounded-lg text-slate-700 leading-relaxed max-h-72 overflow-y-auto border border-slate-200 prose prose-sm max-w-none">
                {detail.transcript ? (
                  <p>{detail.transcript}</p>
                ) : (
                  <p className="text-slate-400 italic">No transcript available.</p>
                )}
              </div>
            </div>

            {/* Ideas Section */}
            {session?.analysis?.ideas && session.analysis.ideas.length > 0 && (
              <div className="mb-8">
                <div className="flex items-center space-x-3 mb-6">
                  <div className="p-2 bg-gradient-to-br from-yellow-100 to-orange-100 rounded-xl">
                    <LightBulbIcon className="w-6 h-6 text-yellow-600" />
                  </div>
                  <div>
                    <h2 className="text-xl font-semibold text-gray-800">Ideas</h2>
                    <p className="text-sm text-gray-500">{session.analysis.ideas.length} creative insights</p>
                  </div>
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  {session.analysis.ideas.map((idea, index) => (
                    <div key={index} className="group relative">
                      {/* Background glow effect */}
                      <div className="absolute inset-0 bg-gradient-to-br from-yellow-200/30 to-orange-200/30 rounded-2xl blur-xl group-hover:blur-2xl transition-all duration-300 opacity-0 group-hover:opacity-100"></div>
                      
                      {/* Card content */}
                      <div className="relative bg-white/80 backdrop-blur-sm border border-yellow-200/50 rounded-2xl p-6 hover:shadow-xl transition-all duration-300 group-hover:border-yellow-300/70">
                        <div className="flex items-start space-x-4">
                          <div className="flex-shrink-0">
                            <div className="w-10 h-10 bg-gradient-to-br from-yellow-400 to-orange-500 rounded-xl flex items-center justify-center shadow-lg">
                              <LightBulbIcon className="w-5 h-5 text-white" />
                            </div>
                          </div>
                          <div className="flex-1 min-w-0">
                            <p className="text-gray-800 font-medium leading-relaxed">{idea}</p>
                            <div className="mt-3 flex items-center text-xs text-gray-500">
                              <span className="bg-yellow-100 text-yellow-700 px-2 py-1 rounded-full font-medium">
                                Idea #{index + 1}
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Tasks Section */}
            {session?.analysis?.tasks && session.analysis.tasks.length > 0 && (
              <div className="mb-8">
                <div className="flex items-center space-x-3 mb-6">
                  <div className="p-2 bg-gradient-to-br from-blue-100 to-indigo-100 rounded-xl">
                    <CheckCircleIcon className="w-6 h-6 text-blue-600" />
                  </div>
                  <div>
                    <h2 className="text-xl font-semibold text-gray-800">Tasks</h2>
                    <p className="text-sm text-gray-500">{session.analysis.tasks.length} action items</p>
                  </div>
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  {session.analysis.tasks.map((task, index) => {
                    const getPriorityColor = (priority: string) => {
                      switch (priority) {
                        case 'Urgent': return 'from-red-500 to-pink-600';
                        case 'High': return 'from-orange-500 to-red-500';
                        case 'Medium': return 'from-yellow-500 to-orange-500';
                        case 'Low': return 'from-green-500 to-blue-500';
                        default: return 'from-gray-500 to-gray-600';
                      }
                    };
                    
                    const getPriorityIcon = (priority: string) => {
                      switch (priority) {
                        case 'Urgent': return <FireIcon className="w-4 h-4" />;
                        case 'High': return <ExclamationTriangleIcon className="w-4 h-4" />;
                        case 'Medium': return <ClockIcon className="w-4 h-4" />;
                        case 'Low': return <CheckCircleIcon className="w-4 h-4" />;
                        default: return <CheckCircleIcon className="w-4 h-4" />;
                      }
                    };
                    
                    return (
                      <div key={index} className="group relative">
                        {/* Background glow effect */}
                        <div className="absolute inset-0 bg-gradient-to-br from-blue-200/30 to-indigo-200/30 rounded-2xl blur-xl group-hover:blur-2xl transition-all duration-300 opacity-0 group-hover:opacity-100"></div>
                        
                        {/* Card content */}
                        <div className="relative bg-white/80 backdrop-blur-sm border border-blue-200/50 rounded-2xl p-6 hover:shadow-xl transition-all duration-300 group-hover:border-blue-300/70">
                          <div className="flex items-start space-x-4">
                            <div className="flex-shrink-0">
                              <div className={`w-10 h-10 bg-gradient-to-br ${getPriorityColor(task.priority)} rounded-xl flex items-center justify-center shadow-lg text-white`}>
                                {getPriorityIcon(task.priority)}
                              </div>
                            </div>
                            <div className="flex-1 min-w-0">
                              <h3 className="text-gray-800 font-semibold mb-2">{task.title}</h3>
                              {task.description && (
                                <p className="text-gray-600 text-sm mb-3 leading-relaxed">{task.description}</p>
                              )}
                              <div className="flex items-center space-x-3">
                                <span className={`inline-flex items-center space-x-1 px-3 py-1 rounded-full text-xs font-medium bg-gradient-to-r ${getPriorityColor(task.priority)} text-white shadow-sm`}>
                                  {getPriorityIcon(task.priority)}
                                  <span>{task.priority}</span>
                                </span>
                                {task.due_date && (
                                  <span className="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded-full">
                                    Due: {new Date(task.due_date).toLocaleDateString()}
                                  </span>
                                )}
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Structured Notes Section */}
            {session?.analysis?.structured_notes && session.analysis.structured_notes.length > 0 && (
              <div className="mb-8">
                <h2 className="text-xl font-semibold text-gray-700 mb-4">Structured Notes</h2>
                <div className="space-y-4">
                  {session.analysis.structured_notes.map((note, index) => {
                    const getNoteTypeIcon = (noteType: string) => {
                      switch (noteType) {
                        case 'Meeting': return <ChatBubbleLeftRightIcon className="w-5 h-5 text-blue-600" />;
                        case 'Brainstorm': return <BrainstormIcon className="w-5 h-5 text-yellow-600" />;
                        case 'Decision': return <DocumentCheckIcon className="w-5 h-5 text-green-600" />;
                        case 'Action': return <PlayIcon className="w-5 h-5 text-red-600" />;
                        case 'Reference': return <BookOpenIcon className="w-5 h-5 text-indigo-600" />;
                        default: return <DocumentCheckIcon className="w-5 h-5 text-gray-600" />;
                      }
                    };
                    
                    return (
                      <div key={index} className="bg-slate-50 border border-slate-200 rounded-xl p-6 hover:shadow-md transition-shadow">
                        <div className="flex items-center space-x-3 mb-3">
                          <div className="p-2 bg-white rounded-lg shadow-sm">
                            {getNoteTypeIcon(note.note_type)}
                          </div>
                          <div>
                            <h3 className="font-semibold text-slate-800">{note.title}</h3>
                            <span className="text-xs text-slate-500 bg-slate-200 px-2 py-1 rounded-full">
                              {note.note_type}
                            </span>
                          </div>
                        </div>
                        <p className="text-slate-700 leading-relaxed mb-3">{note.content}</p>
                        {note.tags && note.tags.length > 0 && (
                          <div className="flex flex-wrap gap-2">
                            {note.tags.map((tag, tagIndex) => (
                              <span key={tagIndex} className="text-xs bg-slate-200 text-slate-600 px-2 py-1 rounded-full">
                                #{tag}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex flex-col sm:flex-row gap-3 mt-10 pt-6 border-t border-slate-200">
              <button 
                onClick={handleExport}
                className="flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white font-medium py-2.5 px-5 rounded-lg transition-colors duration-150 shadow-sm hover:shadow-md w-full sm:w-auto"
              >
                <DocumentArrowDownIcon className="h-5 w-5" />
                Export Data
              </button>
              <button 
                onClick={handleDelete}
                className="flex items-center justify-center gap-2 bg-red-500 hover:bg-red-600 text-white font-medium py-2.5 px-5 rounded-lg transition-colors duration-150 shadow-sm hover:shadow-md w-full sm:w-auto"
              >
                <TrashIcon className="h-5 w-5" />
                Delete
              </button>
            </div>
          </div>
      </main>
    </div>
  );
};

export default Detail;