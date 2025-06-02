import React, { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { LightBulbIcon, ArrowLeftIcon, ClockIcon, TagIcon, SparklesIcon } from "@heroicons/react/24/outline";
import type { VoiceSession } from "../types";
import { fetchSessions } from "../services/api";
import { formatDate } from "../utils";

/**
 * Ideas page component that aggregates and displays all ideas from voice sessions
 * Features:
 * - Aggregated view of all ideas across sessions
 * - Click to navigate to source recording
 * - Search and filter functionality
 */
const Ideas: React.FC = () => {
  const [sessions, setSessions] = useState<VoiceSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const navigate = useNavigate();

  // Load sessions from backend on component mount
  useEffect(() => {
    loadSessions();
  }, []);

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
      setError('Failed to load ideas. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  // Extract all ideas from sessions with source information
  const allIdeas = sessions.flatMap(session => 
    (session.analysis?.ideas || []).map(idea => ({
      id: `${session.id}-${idea}`,
      content: idea,
      sessionId: session.id,
      sessionTitle: session.title,
      date: session.timestamp
    }))
  );

  // Filter ideas based on search query
  const filteredIdeas = allIdeas.filter(idea =>
    idea.content.toLowerCase().includes(searchQuery.toLowerCase()) ||
    idea.sessionTitle.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="min-h-screen bg-gradient-to-br from-yellow-50 via-orange-50 to-pink-50">
      {/* Header */}
      <header className="bg-white/80 backdrop-blur-md shadow-sm border-b border-yellow-200/50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 py-6">
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between space-y-4 sm:space-y-0">
            <div className="flex items-center space-x-4">
              <button
                onClick={() => navigate('/')}
                className="p-2 rounded-xl text-gray-600 hover:text-yellow-600 hover:bg-yellow-100 transition-all duration-200"
              >
                <ArrowLeftIcon className="w-5 h-5" />
              </button>
              <div className="flex items-center space-x-3">
                <div className="p-3 bg-gradient-to-br from-yellow-400 to-orange-500 rounded-2xl shadow-lg">
                  <LightBulbIcon className="w-7 h-7 text-white" />
                </div>
                <div>
                  <h1 className="text-2xl sm:text-3xl font-bold bg-gradient-to-r from-yellow-600 to-orange-600 bg-clip-text text-transparent">Ideas</h1>
                  <p className="text-sm text-gray-600">{filteredIdeas.length} creative insights from {sessions.length} recordings</p>
                </div>
              </div>
            </div>
            
            {/* Search */}
            <div className="relative w-full sm:w-auto">
              <input
                type="text"
                placeholder="Search ideas..."
                className="w-full sm:w-80 pl-4 pr-4 py-3 rounded-xl border border-yellow-200 bg-white/70 backdrop-blur-sm shadow-sm text-sm focus:ring-2 focus:ring-yellow-400 focus:border-transparent outline-none transition-all duration-200"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
          </div>
        </div>
      </header>

      {/* Content */}
      <main className="max-w-7xl mx-auto p-4 sm:p-6">
        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-yellow-600"></div>
            <span className="ml-3 text-gray-600">Loading ideas...</span>
          </div>
        )}

        {/* Error State */}
        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <div className="flex items-center">
              <svg className="w-5 h-5 text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
              </svg>
              <span className="text-red-800">{error}</span>
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
        {!loading && !error && filteredIdeas.length === 0 && (
          <div className="text-center py-12">
            <LightBulbIcon className="w-16 h-16 text-gray-300 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-gray-900 mb-2">No ideas found</h3>
            <p className="text-gray-500 mb-4">
              {searchQuery ? 'Try adjusting your search terms.' : 'Start recording to capture your ideas!'}
            </p>
          </div>
        )}

        {/* Ideas Grid */}
        {!loading && !error && filteredIdeas.length > 0 && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {filteredIdeas.map((idea, index) => (
              <div
                key={idea.id}
                className="group relative cursor-pointer"
                onClick={() => navigate(`/detail/${idea.sessionId}`)}
              >
                {/* Background glow effect */}
                <div className="absolute inset-0 bg-gradient-to-br from-yellow-200/40 to-orange-200/40 rounded-3xl blur-xl group-hover:blur-2xl transition-all duration-500 opacity-0 group-hover:opacity-100"></div>
                
                {/* Card content */}
                <div className="relative bg-white/90 backdrop-blur-sm border border-yellow-200/60 rounded-3xl p-6 hover:shadow-2xl transition-all duration-300 group-hover:border-yellow-300/80 group-hover:bg-white/95">
                  {/* Header with icon and index */}
                  <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center space-x-3">
                      <div className="relative">
                        <div className="absolute inset-0 bg-gradient-to-br from-yellow-400 to-orange-500 rounded-2xl blur-md opacity-50 group-hover:opacity-70 transition-opacity"></div>
                        <div className="relative w-12 h-12 bg-gradient-to-br from-yellow-400 to-orange-500 rounded-2xl flex items-center justify-center shadow-lg">
                          <LightBulbIcon className="w-6 h-6 text-white" />
                        </div>
                      </div>
                      <div>
                        <span className="text-xs font-semibold text-yellow-600 bg-yellow-100 px-2 py-1 rounded-full">
                          Idea #{index + 1}
                        </span>
                      </div>
                    </div>
                    <div className="p-2 rounded-full bg-gray-100 group-hover:bg-yellow-100 transition-colors">
                      <SparklesIcon className="w-4 h-4 text-gray-400 group-hover:text-yellow-500 transition-colors" />
                    </div>
                  </div>
                  
                  {/* Idea content */}
                  <div className="mb-4">
                    <p className="text-gray-800 font-medium leading-relaxed text-base group-hover:text-gray-900 transition-colors">
                      {idea.content}
                    </p>
                  </div>
                  
                  {/* Source information */}
                  <div className="flex items-center justify-between pt-4 border-t border-gray-100">
                    <div className="flex items-center space-x-3">
                      <div className="flex items-center space-x-1 text-sm text-gray-600">
                        <TagIcon className="w-4 h-4" />
                        <span className="font-medium truncate max-w-32">{idea.sessionTitle}</span>
                      </div>
                      <div className="flex items-center space-x-1 text-sm text-gray-500">
                        <ClockIcon className="w-4 h-4" />
                        <span>{formatDate(new Date(idea.date))}</span>
                      </div>
                    </div>
                    <div className="flex-shrink-0">
                      <div className="w-8 h-8 rounded-full bg-gradient-to-r from-yellow-400 to-orange-500 flex items-center justify-center group-hover:scale-110 transition-transform">
                        <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                        </svg>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
};

export default Ideas;