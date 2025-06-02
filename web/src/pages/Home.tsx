import React, { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import Recorder from "../components/Recorder";
import { 
  MagnifyingGlassIcon, 
  PlusIcon, 
  SunIcon, 
  MoonIcon,
  LightBulbIcon,
  CheckCircleIcon,
  SparklesIcon,
  MicrophoneIcon,
  ChatBubbleLeftRightIcon,
  LightBulbIcon as BrainstormIcon,
  DocumentCheckIcon,
  PlayIcon,
  BookOpenIcon
} from "@heroicons/react/24/outline";
import type { Note, VoiceSession } from "../types";
import { formatDate, generateId } from "../utils";
import { fetchSessions, deleteSession, startRecording, stopRecording, getRecordingStatus } from "../services/api";
import { voiceSessionsToNotes, getCategoriesFromSessions } from "../utils/dataTransform";

const Home: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [notes, setNotes] = useState<Note[]>([]);
  const [sessions, setSessions] = useState<VoiceSession[]>([]);
  const [isRecording, setIsRecording] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [activeTab, setActiveTab] = useState<'notes' | 'ideas' | 'tasks'>('notes');
  const [quickRecordingSession, setQuickRecordingSession] = useState<string | null>(null);
  const navigate = useNavigate();

  // Load sessions from backend on component mount
  useEffect(() => {
    loadSessions();
  }, []);

  // Convert sessions to notes when sessions change
  useEffect(() => {
    const convertedNotes = voiceSessionsToNotes(sessions);
    setNotes(convertedNotes);
  }, [sessions]);

  const loadSessions = async () => {
    try {
      setLoading(true);
      setError(null);
      const fetchedSessions = await fetchSessions({
        sort_by: 'timestamp',
        sort_order: 'desc'
      });
      setSessions(fetchedSessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setError('Failed to load recordings. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  // Filter notes based on search query and category
  const filteredNotes = notes.filter(note => {
    const matchesSearch = searchQuery === "" || 
                         note.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         note.content.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         (note.transcript && note.transcript.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesCategory = selectedCategory === "All" || note.tag === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  // Get aggregated ideas and tasks counts
  const totalIdeas = sessions.reduce((count, session) => 
    count + (session.analysis?.ideas?.length || 0), 0
  );
  const totalTasks = sessions.reduce((count, session) => 
    count + (session.analysis?.tasks?.length || 0), 0
  );

  // Get ideas and tasks counts for individual notes
  const getNoteCounts = (noteId: string) => {
    const session = sessions.find(s => s.id === noteId);
    return {
      ideas: session?.analysis?.ideas?.length || 0,
      tasks: session?.analysis?.tasks?.length || 0
    };
  };

  // Get note type icon based on structured notes
  const getNoteTypeIcon = (sessionId: string) => {
    const session = sessions.find(s => s.id === sessionId);
    if (!session?.analysis?.structured_notes?.length) {
      return <MicrophoneIcon className="w-6 h-6 text-purple-600" />;
    }
    
    // Get the most common note type or the first one
    const noteType = session.analysis.structured_notes[0].note_type;
    
    switch (noteType) {
      case 'Meeting':
        return <ChatBubbleLeftRightIcon className="w-6 h-6 text-blue-600" />;
      case 'Brainstorm':
        return <BrainstormIcon className="w-6 h-6 text-yellow-600" />;
      case 'Decision':
        return <DocumentCheckIcon className="w-6 h-6 text-green-600" />;
      case 'Action':
        return <PlayIcon className="w-6 h-6 text-red-600" />;
      case 'Reference':
        return <BookOpenIcon className="w-6 h-6 text-indigo-600" />;
      default:
        return <MicrophoneIcon className="w-6 h-6 text-purple-600" />;
    }
  };

  // Handle quick record button click
  const handleQuickRecord = async () => {
    if (isRecording) {
      // Stop recording
      try {
        setIsUploading(true);
        setError(null);
        
        if (quickRecordingSession) {
          try {
            await stopRecording();
          } catch (apiError) {
            console.warn('API stop recording failed, continuing with local cleanup:', apiError);
            // Continue with local cleanup even if API fails
          }
          
          // Always reload sessions to get the latest data
          try {
            await loadSessions();
          } catch (loadError) {
            console.warn('Failed to reload sessions:', loadError);
          }
        }
        
        setIsRecording(false);
        setQuickRecordingSession(null);
      } catch (error) {
        console.error('Error stopping quick recording:', error);
        setError('停止录音失败，请重试。');
        // Still cleanup the recording state
        setIsRecording(false);
        setQuickRecordingSession(null);
      } finally {
        setIsUploading(false);
      }
    } else {
      // Start recording
      try {
        setError(null);
        const sessionId = await startRecording();
        setQuickRecordingSession(sessionId);
        setIsRecording(true);
      } catch (error) {
        console.error('Error starting quick recording:', error);
        setError('开始录音失败，请检查麦克风权限。');
      }
    }
  };

  // Handle upload from recorder (for modal recording)
  const handleUpload = async (session: VoiceSession) => {
    try {
      setIsUploading(true);
      setError(null);
      
      // Add the new session to the current sessions list
      setSessions(prevSessions => [session, ...prevSessions]);
      
      // Close the recording modal
      setIsRecording(false);
      
      // Show success message
      console.log('Recording processed successfully:', session.title);
    } catch (error) {
      console.error('Error handling upload:', error);
      setError('Failed to process recording. Please try again.');
    } finally {
      setIsUploading(false);
    }
  };

  // Handle category selection
  const handleCategorySelect = (category: string) => {
    setSelectedCategory(category);
  };

  // Get unique categories from sessions
  const categories = ["All", ...getCategoriesFromSessions(sessions)];
  
  const themeClasses = isDarkMode 
    ? "min-h-screen bg-gray-900 flex flex-col text-white"
    : "min-h-screen bg-gray-50 flex flex-col";
    
  const headerClasses = isDarkMode
    ? "sticky top-0 z-10 bg-gray-800 border-b border-gray-700"
    : "sticky top-0 z-10 bg-white border-b border-gray-200";

  return (
    <div className={themeClasses}>
      {/* Header */}
      <header className={headerClasses}>
        <div className="max-w-6xl mx-auto px-6 py-6">
          <div className="flex items-center justify-between">
            {/* Logo and Brand */}
            <div className="flex items-center space-x-4">
              <div className="flex items-center space-x-3">
                <div className="relative">
                  <div className="w-10 h-10 bg-gradient-to-br from-purple-500 to-pink-500 rounded-xl flex items-center justify-center shadow-lg">
                    <MicrophoneIcon className="w-6 h-6 text-white" />
                  </div>
                  <div className="absolute -top-1 -right-1 w-4 h-4 bg-green-400 rounded-full border-2 border-white animate-pulse"></div>
                </div>
                <div>
                  <h1 className={`text-2xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent`}>
                    VoiceNotes
                  </h1>
                  <p className={`text-xs ${isDarkMode ? 'text-gray-400' : 'text-gray-500'}`}>Capture your thoughts</p>
                </div>
              </div>
            </div>

            {/* Search and Controls */}
            <div className="flex items-center space-x-4">
              <div className="relative">
                <MagnifyingGlassIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
                <input
                  type="text"
                  placeholder="Search notes, ideas, tasks..."
                  className={`pl-10 pr-4 py-2.5 rounded-xl border shadow-sm text-sm focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none w-80 transition-all ${
                    isDarkMode 
                      ? 'border-gray-600 bg-gray-700 text-white placeholder-gray-400' 
                      : 'border-gray-300 bg-white text-gray-900 placeholder-gray-500'
                  }`}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
              
              {/* Theme Toggle */}
              <button 
                onClick={() => setIsDarkMode(!isDarkMode)}
                className={`p-2.5 rounded-xl transition-all duration-200 ${
                  isDarkMode 
                    ? 'text-yellow-400 hover:bg-gray-700 bg-gray-600' 
                    : 'text-gray-600 hover:bg-gray-100 bg-gray-50'
                }`}
                title={isDarkMode ? 'Switch to Light Mode' : 'Switch to Dark Mode'}
              >
                {isDarkMode ? <SunIcon className="w-6 h-6" /> : <MoonIcon className="w-6 h-6" />}
              </button>
            </div>
          </div>

          {/* Navigation Tabs */}
          <div className="flex items-center space-x-1 mt-6">
            <button
              onClick={() => setActiveTab('notes')}
              className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                activeTab === 'notes'
                  ? isDarkMode 
                    ? 'bg-purple-600 text-white' 
                    : 'bg-purple-100 text-purple-700'
                  : isDarkMode 
                    ? 'text-gray-400 hover:text-white hover:bg-gray-700' 
                    : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
              }`}
            >
              <SparklesIcon className="w-4 h-4" />
              <span>Notes</span>
              <span className={`px-2 py-0.5 rounded-full text-xs ${
                activeTab === 'notes'
                  ? isDarkMode ? 'bg-purple-500 text-white' : 'bg-purple-200 text-purple-800'
                  : isDarkMode ? 'bg-gray-600 text-gray-300' : 'bg-gray-200 text-gray-600'
              }`}>
                {notes.length}
              </span>
            </button>
            
            <button
              onClick={() => navigate('/ideas')}
              className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                isDarkMode 
                  ? 'text-gray-400 hover:text-white hover:bg-gray-700' 
                  : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
              }`}
            >
              <LightBulbIcon className="w-4 h-4" />
              <span>Ideas</span>
              <span className={`px-2 py-0.5 rounded-full text-xs ${
                isDarkMode ? 'bg-yellow-600 text-yellow-100' : 'bg-yellow-100 text-yellow-800'
              }`}>
                {totalIdeas}
              </span>
            </button>
            
            <button
              onClick={() => navigate('/tasks')}
              className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                isDarkMode 
                  ? 'text-gray-400 hover:text-white hover:bg-gray-700' 
                  : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
              }`}
            >
              <CheckCircleIcon className="w-4 h-4" />
              <span>Tasks</span>
              <span className={`px-2 py-0.5 rounded-full text-xs ${
                isDarkMode ? 'bg-blue-600 text-blue-100' : 'bg-blue-100 text-blue-800'
              }`}>
                {totalTasks}
              </span>
            </button>
          </div>
        </div>
      </header>
      <main className="max-w-6xl mx-auto p-6 flex-grow">
        {/* Quick Record Button - Golden Microphone */} 
        <button 
          onClick={handleQuickRecord}
          className={`fixed bottom-8 right-8 rounded-full p-4 shadow-xl transition-all duration-200 transform hover:scale-110 z-20 group ${
            isRecording && quickRecordingSession
              ? 'bg-red-500 hover:bg-red-600 animate-pulse' 
              : 'bg-gradient-to-r from-yellow-400 to-yellow-600 hover:from-yellow-500 hover:to-yellow-700'
          } text-white`}
          title={isRecording && quickRecordingSession ? "正在录音..." : "快速录音"}
          disabled={isUploading || (isRecording && !quickRecordingSession)}
        >
          {isRecording && quickRecordingSession ? (
            <div className="w-8 h-8 flex items-center justify-center">
              <div className="w-4 h-4 bg-white rounded-sm"></div>
            </div>
          ) : (
            <MicrophoneIcon className="w-8 h-8" />
          )}
        </button>

        {/* Quick Recording Status - Show when quick recording is active */}
        {isRecording && quickRecordingSession && (
          <div className="fixed top-4 left-1/2 transform -translate-x-1/2 z-30">
            <div className={`rounded-lg shadow-lg p-4 flex items-center space-x-3 ${
              isDarkMode ? 'bg-gray-800 text-white border border-gray-700' : 'bg-white text-gray-800 border border-gray-200'
            }`}>
              <div className="w-3 h-3 bg-red-500 rounded-full animate-pulse"></div>
              <span className="font-medium">正在录音...</span>
              <span className="text-sm text-gray-500">点击话筒停止</span>
            </div>
          </div>
        )}

        {/* Traditional Recording Modal - Show when isRecording is true but not quick recording */} 
        {isRecording && !quickRecordingSession && (
          <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-30 p-4">
            <div className={`rounded-2xl shadow-2xl p-8 w-full max-w-lg ${
              isDarkMode ? 'bg-gray-800 text-white' : 'bg-white text-gray-800'
            }`}>
              <h2 className="text-2xl font-semibold mb-6 text-center">创建新录音</h2>
              <Recorder onUpload={handleUpload} />
              <button
                className={`mt-6 w-full font-medium py-2.5 px-6 rounded-lg transition-colors duration-150 ${
                  isDarkMode 
                    ? 'bg-gray-700 hover:bg-gray-600 text-gray-300' 
                    : 'bg-gray-100 hover:bg-gray-200 text-gray-700'
                }`}
                onClick={() => setIsRecording(false)}
              >
                取消
              </button>
            </div>
          </div>
        )}

        {/* Category Filters */}
        <div className="mb-6">
          <div className="flex gap-2 overflow-x-auto pb-2">
            {categories.map((category) => (
              <button
                key={category}
                className={`px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-150 ${
                  selectedCategory === category
                    ? isDarkMode 
                      ? 'bg-purple-600 text-white border border-purple-600 hover:bg-purple-700'
                      : 'bg-purple-100 text-purple-700 border border-purple-200 hover:bg-purple-200'
                    : isDarkMode 
                      ? 'bg-gray-700 text-gray-300 border border-gray-600 hover:bg-gray-600'
                      : 'bg-white text-gray-600 border border-gray-200 hover:bg-gray-50'
                }`}
                onClick={() => handleCategorySelect(category)}
              >
                {category}
              </button>
            ))}
          </div>
        </div>

        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600"></div>
            <span className={`ml-3 ${isDarkMode ? 'text-gray-300' : 'text-gray-600'}`}>Loading recordings...</span>
          </div>
        )}

        {/* Error State */}
        {error && (
          <div className={`border rounded-lg p-4 mb-6 ${
            isDarkMode 
              ? 'bg-red-900/20 border-red-800 text-red-300' 
              : 'bg-red-50 border-red-200 text-red-800'
          }`}>
            <div className="flex items-center">
              <svg className="w-5 h-5 text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
              </svg>
              <span>{error}</span>
              <button 
                onClick={loadSessions}
                className="ml-auto text-red-600 hover:text-red-800 text-sm font-medium"
              >
                Retry
              </button>
            </div>
          </div>
        )}

        {/* Empty State */}
        {!loading && !error && filteredNotes.length === 0 && (
          <div className="text-center py-12">
            <MicrophoneIcon className={`w-16 h-16 mx-auto mb-4 ${isDarkMode ? 'text-gray-600' : 'text-gray-300'}`} />
            <h3 className={`text-lg font-medium mb-2 ${isDarkMode ? 'text-gray-200' : 'text-gray-900'}`}>
              {searchQuery ? 'No matching recordings' : 'No recordings yet'}
            </h3>
            <p className={`mb-4 ${isDarkMode ? 'text-gray-400' : 'text-gray-500'}`}>
              {searchQuery ? 'Try adjusting your search terms.' : 'Start recording your first voice note!'}
            </p>
            {!searchQuery && (
              <button 
                onClick={() => setIsRecording(true)}
                className="inline-flex items-center px-6 py-3 bg-gradient-to-r from-purple-600 to-pink-600 text-white rounded-lg hover:from-purple-700 hover:to-pink-700 transition-all transform hover:scale-105 shadow-lg"
              >
                <PlusIcon className="w-5 h-5 mr-2" />
                Start Recording
              </button>
            )}
          </div>
        )}

        {/* Notes Grid */}
        {!loading && !error && filteredNotes.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {filteredNotes.map((note) => {
              const counts = getNoteCounts(note.id);
              return (
                <div
                  key={note.id}
                  className={`rounded-xl shadow-sm hover:shadow-lg transition-all duration-200 cursor-pointer overflow-hidden border group ${
                    isDarkMode 
                      ? 'bg-gray-800 border-gray-700 hover:border-gray-600' 
                      : 'bg-white border-gray-200 hover:border-gray-300'
                  }`}
                  onClick={() => navigate(`/detail/${note.id}`)}
                >
                  {/* Compact Image Area */}
                  <div className="h-32 bg-gradient-to-br from-purple-100 to-pink-100 flex items-center justify-center relative">
                    <div className="w-12 h-12 bg-white rounded-full flex items-center justify-center shadow-sm group-hover:scale-110 transition-transform">
                      {getNoteTypeIcon(note.id)}
                    </div>
                    
                    {/* Ideas and Tasks Badges */}
                    <div className="absolute top-2 right-2 flex space-x-1">
                      {counts.ideas > 0 && (
                        <div className="flex items-center space-x-1 bg-yellow-100 text-yellow-800 px-2 py-1 rounded-full text-xs font-medium">
                          <LightBulbIcon className="w-3 h-3" />
                          <span>{counts.ideas}</span>
                        </div>
                      )}
                      {counts.tasks > 0 && (
                        <div className="flex items-center space-x-1 bg-blue-100 text-blue-800 px-2 py-1 rounded-full text-xs font-medium">
                          <CheckCircleIcon className="w-3 h-3" />
                          <span>{counts.tasks}</span>
                        </div>
                      )}
                    </div>
                  </div>
                  
                  {/* Compact Content */}
                  <div className="p-4">
                    <div className="flex items-center justify-between mb-2">
                      <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                        isDarkMode 
                          ? 'bg-purple-900/50 text-purple-300' 
                          : 'bg-purple-100 text-purple-700'
                      }`}>
                        {note.tag}
                      </span>
                      <span className={`text-xs ${isDarkMode ? 'text-gray-400' : 'text-gray-500'}`}>
                        {note.duration}
                      </span>
                    </div>
                    
                    <h3 className={`font-semibold mb-2 line-clamp-2 text-sm ${
                      isDarkMode ? 'text-gray-200' : 'text-gray-900'
                    }`}>
                      {note.title}
                    </h3>
                    
                    <p className={`text-xs line-clamp-2 mb-2 ${
                      isDarkMode ? 'text-gray-400' : 'text-gray-600'
                    }`}>
                      {note.content}
                    </p>
                    
                    <div className={`text-xs ${isDarkMode ? 'text-gray-500' : 'text-gray-400'}`}>
                      {formatDate(new Date(note.date))}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </main>
    </div>
  );
};

export default Home;